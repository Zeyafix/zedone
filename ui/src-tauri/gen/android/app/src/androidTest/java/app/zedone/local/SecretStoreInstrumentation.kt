package app.zedone.local

import android.app.Activity
import android.app.Instrumentation
import android.os.Bundle
import android.security.keystore.KeyInfo
import java.io.File
import java.security.KeyStore
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.atomic.AtomicInteger
import javax.crypto.SecretKeyFactory

// Native test runner: no Activity, UI, production namespace or input/output keys.
class SecretStoreInstrumentation : Instrumentation() {
  override fun onCreate(arguments: Bundle?) { super.onCreate(arguments); start() }
  override fun onStart() {
    val namespace = MessageDigest.getInstance("SHA-256").digest(UUID.randomUUID().toString().toByteArray())
      .joinToString("") { "%02x".format(it) }
    val store = ProtectedSecrets(targetContext, namespace)
    val directory = File(targetContext.noBackupFilesDir, "host-secrets/$namespace")
    val baseFile = File(directory, "${ProtectedSecrets.SLOT}.bin")
    val alias = "app.zedone.local/$namespace/${ProtectedSecrets.SLOT}"
    val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    fun fails(action: () -> Unit) {
      check(runCatching(action).isFailure)
    }
    val result = Bundle()
    var succeeded = false
    var stage = "missing-and-limits"
    try {
      check(store.load() == null)
      fails { store.create(ByteArray(0)) }
      fails { store.create(ByteArray(2049)) }
      val secret = ByteArray(2048) { (it % 251).toByte() }
      stage = "persistence-and-keystore"
      store.create(secret)
      val encoded = baseFile.readBytes()
      check(!encoded.contentEquals(secret))
      check(!encoded.toList().windowed(64).contains(secret.take(64)))
      check(ProtectedSecrets(targetContext, namespace).load()!!.contentEquals(secret))
      fails { store.create(byteArrayOf(2)) }
      check(store.load()!!.contentEquals(secret))
      val key = keyStore.getKey(alias, null) as javax.crypto.SecretKey
      check(key.encoded == null)
      val info = SecretKeyFactory.getInstance(key.algorithm, "AndroidKeyStore")
        .getKeySpec(key, KeyInfo::class.java) as KeyInfo
      check(info.keySize == 256)
      check(info.blockModes.contentEquals(arrayOf("GCM")))
      // Authenticated ciphertext, header/length, and missing key all fail closed.
      stage = "corruption"
      encoded[encoded.lastIndex] = (encoded.last().toInt() xor 1).toByte()
      baseFile.writeBytes(encoded)
      fails { store.load() }; fails { store.create(secret) }
      check(baseFile.readBytes().contentEquals(encoded))
      store.remove()
      check(store.load() == null && !keyStore.containsAlias(alias))
      // Concurrent provisioning has one winner and no implicit replacement.
      val ready = CountDownLatch(1)
      val count = AtomicInteger(0)
      val unexpected = AtomicInteger(0)
      val threads = (1..4).map { value -> Thread {
        ready.await()
        try { ProtectedSecrets(targetContext, namespace).create(byteArrayOf(value.toByte())); count.incrementAndGet() }
        catch (error: SecretFailure) { if (error.code != "already_exists") unexpected.incrementAndGet() }
        catch (_: Throwable) { unexpected.incrementAndGet() }
      }.apply { start() } }
      ready.countDown(); threads.forEach { it.join() }
      check(count.get() == 1 && unexpected.get() == 0)
      val first = store.load()!!
      check(first.size == 1 && first[0].toInt() in 1..4)
      // AtomicFile restart recovery of the last completed value.
      stage = "atomic-recovery"
      baseFile.copyTo(File(baseFile.path + ".bak"))
      baseFile.writeBytes(byteArrayOf(0))
      check(ProtectedSecrets(targetContext, namespace).load()!!.contentEquals(first))
      keyStore.deleteEntry(alias)
      stage = "key-loss-and-removal"
      fails { store.load() }; fails { store.create(secret) }
      store.remove(); store.remove()
      check(store.load() == null)
      store.create(secret)
      check(!baseFile.readBytes().contentEquals(encoded))
      store.remove()
      succeeded = true
      result.putString("zedone_secrets", "passed")
    } catch (_: Throwable) {
      // Never stringify exceptions, test values or native error details.
      result.putString("zedone_secrets", "failed")
      result.putString("stage", stage)
    } finally {
      runCatching { store.remove() }
      check(directory.canonicalPath.startsWith(targetContext.noBackupFilesDir.canonicalPath + File.separator))
      directory.deleteRecursively()
      if (keyStore.containsAlias(alias) || directory.exists()) {
        succeeded = false; result.putString("zedone_secrets", "cleanup_failed")
      }
      finish(if (succeeded) Activity.RESULT_OK else Activity.RESULT_CANCELED, result)
    }
  }
}
