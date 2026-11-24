package app.zedone.local

import android.app.Activity
import android.content.Intent
import android.provider.DocumentsContract
import androidx.activity.result.ActivityResult
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.util.concurrent.Executors

@InvokeArg
class SnapshotFileArgs {
  var archive: String? = null
  var limit: Int = 0
}

@TauriPlugin
class SnapshotFilesPlugin(private val activity: Activity) : Plugin(activity) {
  private val worker = Executors.newSingleThreadExecutor()
  private var active = false

  override fun onDestroy(activity: androidx.appcompat.app.AppCompatActivity) {
    worker.shutdown()
  }

  @Command
  fun select(invoke: Invoke) {
    activity.runOnUiThread {
      if (active) { invoke.resolve(JSObject().put("status", "unavailable")); return@runOnUiThread }
      try {
        val args = invoke.parseArgs(SnapshotFileArgs::class.java)
        require(args.limit > 0)
        args.archive?.let { require(it.isNotEmpty() && it.length <= args.limit * 2 && it.length % 2 == 0 && it.all { c -> c in '0'..'9' || c in 'a'..'f' }) }
        val intent = Intent(if (args.archive == null) Intent.ACTION_OPEN_DOCUMENT else Intent.ACTION_CREATE_DOCUMENT).apply {
          addCategory(Intent.CATEGORY_OPENABLE)
          type = if (args.archive == null) "*/*" else "application/octet-stream"
          if (args.archive != null) putExtra(Intent.EXTRA_TITLE, "space.zedspace")
        }
        active = true
        startActivityForResult(invoke, intent, "selected")
      } catch (_: Exception) {
        active = false
        invoke.resolve(JSObject().put("status", "unavailable"))
      }
    }
  }

  @ActivityCallback
  private fun selected(invoke: Invoke, result: ActivityResult) {
    if (result.resultCode != Activity.RESULT_OK) {
      active = false
      invoke.resolve(JSObject().put("status", "cancelled"))
      return
    }
    worker.execute {
      val reply = JSObject()
      val uri = result.data?.data
      val args = invoke.parseArgs(SnapshotFileArgs::class.java)
      try {
        require(uri != null && uri.scheme == "content")
        val resolver = activity.contentResolver
        if (args.archive == null) {
          val bytes = resolver.openInputStream(uri)?.use { input ->
            val output = java.io.ByteArrayOutputStream()
            val buffer = ByteArray(8192)
            while (output.size() <= args.limit) {
              val count = input.read(buffer, 0, minOf(buffer.size, args.limit + 1 - output.size()))
              if (count < 0) break
              if (count == 0) throw java.io.IOException()
              output.write(buffer, 0, count)
            }
            output.toByteArray()
          } ?: throw java.io.IOException()
          try {
            if (bytes.isEmpty() || bytes.size > args.limit) reply.put("status", "invalid_file")
            else {
              val digits = "0123456789abcdef"
              val hex = CharArray(bytes.size * 2)
              bytes.forEachIndexed { i, b -> val n = b.toInt() and 255; hex[i * 2] = digits[n ushr 4]; hex[i * 2 + 1] = digits[n and 15] }
              reply.put("status", "ok").put("archive", String(hex))
            }
          } finally { bytes.fill(0) }
        } else {
          val hex = args.archive!!
          val bytes = ByteArray(hex.length / 2) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
          try {
            resolver.openOutputStream(uri, "w")?.use { it.write(bytes); it.flush() } ?: throw java.io.IOException()
            reply.put("status", "ok")
          } finally { bytes.fill(0) }
        }
      } catch (_: Exception) {
        if (args.archive != null && uri != null) {
          try { DocumentsContract.deleteDocument(activity.contentResolver, uri) } catch (_: Exception) { }
        }
        reply.put("status", "unavailable")
      }
      activity.runOnUiThread { active = false; invoke.resolve(reply) }
    }
  }
}
