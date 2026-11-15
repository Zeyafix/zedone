package app.zedone.local

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import java.io.File
import java.io.RandomAccessFile
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

// Host-only primitive: no Activity, exported component, plaintext fallback or logging.
class SecretFailure(val code: String) : Exception(code)

class ProtectedSecrets(context: Context, private val namespace: String, private val slot: String = SLOT) {
  companion object {
    private val operations = Any()
    const val SLOT = "channel-identity-v1"
    const val PENDING_SLOT = "pending-pairing-v1"
    const val CONNECTED_SLOT = "connected-pairing-v1"
    const val LIMIT = 2048
  }
  init {
    require(namespace.matches(Regex("[a-f0-9]{64}")))
    require(slot == SLOT || slot == PENDING_SLOT || slot == CONNECTED_SLOT)
  }
  private val directory = File(context.noBackupFilesDir, "host-secrets/$namespace")
  private val alias = "app.zedone.local/$namespace/$slot"
  private val file get() = AtomicFile(File(directory, "$slot.bin"))
  private val aad get() = "zedone/host-secret/v1\u0000$namespace/$slot".toByteArray(Charsets.UTF_8)
  private fun keyStore() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

  private fun <T> locked(action: () -> T): T = synchronized(operations) {
    try {
      check(directory.isDirectory || directory.mkdirs())
      RandomAccessFile(File(directory, "operations.lock"), "rw").use { lockFile ->
        lockFile.channel.lock().use { action() }
      }
    } catch (error: SecretFailure) { throw error }
    catch (_: Exception) { throw SecretFailure("unavailable") }
  }

  private fun read(): ByteArray? {
    // AtomicFile also recovers an interrupted write. Do not conflate corruption
    // or a lost Keystore key with a missing record.
    val input = try { file.openRead() } catch (error: java.io.FileNotFoundException) {
      if (file.baseFile.exists() || File(file.baseFile.path + ".bak").exists()) throw error
      return null
    }
    val encoded = input.use { it.readBytesBounded(LIMIT + 29) }
    if (encoded.size < 30 || encoded[0] != 1.toByte()) throw SecretFailure("invalid_secret")
    val key = keyStore().getKey(alias, null) as? SecretKey ?: throw SecretFailure("unavailable")
    val cipher = Cipher.getInstance("AES/GCM/NoPadding")
    cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, encoded.copyOfRange(1, 13)))
    cipher.updateAAD(aad)
    val result = cipher.doFinal(encoded, 13, encoded.size - 13)
    if (result.isEmpty() || result.size > LIMIT) {
      result.fill(0)
      throw SecretFailure("invalid_secret")
    }
    return result
  }

  fun load(): ByteArray? = locked { read() }

  fun create(secret: ByteArray) = locked {
    if (secret.isEmpty() || secret.size > LIMIT) throw SecretFailure("invalid_secret")
    // Check even an unreadable existing record: never replace it implicitly.
    val previous = read()
    if (previous != null) {
      previous.fill(0)
      throw SecretFailure("already_exists")
    }
    val store = keyStore()
    val key = (store.getKey(alias, null) as? SecretKey) ?: KeyGenerator.getInstance(
      KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore"
    ).apply {
      init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
        .setKeySize(256).setBlockModes(KeyProperties.BLOCK_MODE_GCM)
        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
        .setRandomizedEncryptionRequired(true).build())
    }.generateKey()
    val cipher = Cipher.getInstance("AES/GCM/NoPadding")
    cipher.init(Cipher.ENCRYPT_MODE, key)
    check(cipher.iv.size == 12)
    cipher.updateAAD(aad)
    val encoded = byteArrayOf(1) + cipher.iv + cipher.doFinal(secret)
    val output = file.startWrite()
    try {
      output.write(encoded)
      file.finishWrite(output)
    } catch (error: Exception) {
      file.failWrite(output)
      throw error
    }
  }

  fun remove() = locked {
    file.delete()
    check(!file.baseFile.exists() && !File(file.baseFile.path + ".bak").exists())
    // Delete ciphertext before the key: a failed file deletion must not silently
    // turn a readable record into an unrecoverable one.
    keyStore().deleteEntry(alias)
  }

  private fun java.io.InputStream.readBytesBounded(limit: Int): ByteArray {
    val bytes = ByteArray(limit + 1)
    var size = 0
    while (size < bytes.size) {
      val count = read(bytes, size, bytes.size - size)
      if (count < 0) break
      size += count
    }
    if (size > limit) throw SecretFailure("invalid_secret")
    return bytes.copyOf(size)
  }
}
