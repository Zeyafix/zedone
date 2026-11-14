package app.zedone.local

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class SecretArgs {
  lateinit var namespace: String
  lateinit var operation: String
  lateinit var slot: String
  var bytes: List<Int>? = null
}

@TauriPlugin
class SecretsPlugin(private val activity: Activity) : Plugin(activity) {
  // Rust's plugin dispatcher rejects all renderer calls. This command is invoked
  // only by the Rust host handle, never registered as an application command.
  @Command
  fun access(invoke: Invoke) {
    val result = JSObject()
    try {
      val args = invoke.parseArgs(SecretArgs::class.java)
      if (args.slot != ProtectedSecrets.SLOT && args.slot != ProtectedSecrets.PENDING_SLOT) throw SecretFailure("invalid_secret")
      val store = ProtectedSecrets(activity, args.namespace, args.slot)
      when (args.operation) {
        "load" -> {
          val secret = store.load()
          result.put("status", if (secret == null) "missing" else "ok")
          if (secret != null) {
            try { result.put("bytes", org.json.JSONArray(secret.map { it.toInt() and 255 })) }
            finally { secret.fill(0) }
          }
        }
        "create" -> {
          val values = args.bytes ?: throw SecretFailure("invalid_secret")
          if (values.size !in 1..ProtectedSecrets.LIMIT || values.any { it !in 0..255 }) throw SecretFailure("invalid_secret")
          val bytes = values.map { it.toByte() }.toByteArray()
          try { store.create(bytes) } finally { bytes.fill(0) }
          result.put("status", "ok")
        }
        "remove" -> { store.remove(); result.put("status", "ok") }
        else -> throw SecretFailure("invalid_secret")
      }
    } catch (error: SecretFailure) { result.put("status", error.code) }
    catch (_: Exception) { result.put("status", "unavailable") }
    invoke.resolve(result)
  }
}
