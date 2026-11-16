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

class SecretStoreInstrumentation : Instrumentation() {
  override fun onCreate(arguments: Bundle?) { super.onCreate(arguments); start() }
  override fun onStart() {
    val namespace = MessageDigest.getInstance("SHA-256").digest(UUID.randomUUID().toString().toByteArray())
      .joinToString("") { "%02x".format(it) }
    val store = ProtectedSecrets(targetContext, namespace)
    val pending = ProtectedSecrets(targetContext, namespace, ProtectedSecrets.PENDING_SLOT)
    val connected = ProtectedSecrets(targetContext, namespace, ProtectedSecrets.CONNECTED_SLOT)
    val outgoing = ProtectedSecrets(targetContext, namespace, ProtectedSecrets.OUTGOING_SLOT)
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
      fails { ProtectedSecrets(targetContext, namespace, "../invalid") }
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
      stage = "pending-slot-isolation"
      val pendingSecret = ByteArray(1024) { ((it + 17) % 251).toByte() }
      check(pending.load() == null)
      pending.create(pendingSecret)
      check(ProtectedSecrets(targetContext, namespace, ProtectedSecrets.PENDING_SLOT).load()!!.contentEquals(pendingSecret))
      val pendingFile = File(directory, "${ProtectedSecrets.PENDING_SLOT}.bin")
      val pendingBytes = pendingFile.readBytes()
      pendingFile.writeBytes(baseFile.readBytes())
      fails { pending.load() }; fails { pending.create(pendingSecret) }
      pendingFile.writeBytes(pendingBytes)
      stage = "connected-slot-isolation"
      check(connected.load() == null)
      connected.create(pendingSecret)
      check(ProtectedSecrets(targetContext, namespace, ProtectedSecrets.CONNECTED_SLOT).load()!!.contentEquals(pendingSecret))
      fails { connected.create(byteArrayOf(2)) }
      val connectedFile = File(directory, "${ProtectedSecrets.CONNECTED_SLOT}.bin")
      val connectedBytes = connectedFile.readBytes()
      connectedFile.writeBytes(pendingBytes)
      fails { connected.load() }; fails { connected.create(pendingSecret) }
      connectedFile.writeBytes(connectedBytes)
      pending.remove()
      check(pending.load() == null && store.load()!!.contentEquals(secret))
      check(connected.load()!!.contentEquals(pendingSecret))
      connected.remove()
      check(connected.load() == null && store.load()!!.contentEquals(secret))
      stage = "outgoing-slot-isolation"
      check(outgoing.load() == null)
      outgoing.create(pendingSecret)
      check(ProtectedSecrets(targetContext, namespace, ProtectedSecrets.OUTGOING_SLOT).load()!!.contentEquals(pendingSecret))
      val outgoingFile = File(directory, "${ProtectedSecrets.OUTGOING_SLOT}.bin")
      val outgoingBytes = outgoingFile.readBytes()
      outgoingFile.writeBytes(pendingBytes)
      fails { outgoing.load() }; fails { outgoing.create(pendingSecret) }
      outgoingFile.writeBytes(outgoingBytes)
      outgoing.remove()
      check(outgoing.load() == null)
      val key = keyStore.getKey(alias, null) as javax.crypto.SecretKey
      check(key.encoded == null)
      val info = SecretKeyFactory.getInstance(key.algorithm, "AndroidKeyStore")
        .getKeySpec(key, KeyInfo::class.java) as KeyInfo
      check(info.keySize == 256)
      check(info.blockModes.contentEquals(arrayOf("GCM")))
      stage = "corruption"
      encoded[encoded.lastIndex] = (encoded.last().toInt() xor 1).toByte()
      baseFile.writeBytes(encoded)
      fails { store.load() }; fails { store.create(secret) }
      check(baseFile.readBytes().contentEquals(encoded))
      store.remove()
      check(store.load() == null && !keyStore.containsAlias(alias))
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
      result.putString("zedone_secrets", "failed")
      result.putString("stage", stage)
    } finally {
      runCatching { store.remove() }
      runCatching { pending.remove() }
      runCatching { connected.remove() }
      runCatching { outgoing.remove() }
      check(directory.canonicalPath.startsWith(targetContext.noBackupFilesDir.canonicalPath + File.separator))
      directory.deleteRecursively()
      if (keyStore.containsAlias(alias) || keyStore.containsAlias("app.zedone.local/$namespace/${ProtectedSecrets.PENDING_SLOT}") || keyStore.containsAlias("app.zedone.local/$namespace/${ProtectedSecrets.CONNECTED_SLOT}") || keyStore.containsAlias("app.zedone.local/$namespace/${ProtectedSecrets.OUTGOING_SLOT}") || directory.exists()) {
        succeeded = false; result.putString("zedone_secrets", "cleanup_failed")
      }
      finish(if (succeeded) Activity.RESULT_OK else Activity.RESULT_CANCELED, result)
    }
  }
}
