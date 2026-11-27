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

  @Command
  fun selectFolder(invoke: Invoke) {
    activity.runOnUiThread {
      if(active) { invoke.resolve(JSObject().put("status","unavailable"));return@runOnUiThread }
      try {
        val args=invoke.parseArgs(BackupFolderArgs::class.java)
        val intent=Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
          addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
          if(args.writable) addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        }
        active=true
        startActivityForResult(invoke,intent,"folderSelected")
      } catch (_: Exception) { active=false;invoke.resolve(JSObject().put("status","unavailable")) }
    }
  }

  @ActivityCallback
  private fun folderSelected(invoke: Invoke,result: ActivityResult) {
    val reply=JSObject()
    try {
      if(result.resultCode!=Activity.RESULT_OK) reply.put("status","cancelled") else {
        val uri=result.data?.data ?: throw java.io.IOException()
        require(uri.scheme=="content" && uri.authority in setOf("com.android.externalstorage.documents","com.android.providers.downloads.documents"))
        val args=invoke.parseArgs(BackupFolderArgs::class.java)
        val flags=Intent.FLAG_GRANT_READ_URI_PERMISSION or if(args.writable) Intent.FLAG_GRANT_WRITE_URI_PERMISSION else 0
        require((result.data!!.flags and flags)==flags)
        activity.contentResolver.takePersistableUriPermission(uri,flags)
        val canonical=DocumentsContract.buildTreeDocumentUri(uri.authority,DocumentsContract.getTreeDocumentId(uri))
        val root="saf-"+canonical.toString().toByteArray(Charsets.UTF_8).joinToString("") { "%02x".format(it.toInt() and 255) }
        reply.put("status","ok").put("root",root)
      }
    } catch (_: Exception) { reply.put("status","unavailable") }
    active=false
    invoke.resolve(reply)
  }

  @Command
  fun tree(invoke: Invoke) {
    worker.execute {
      try { invoke.resolve(BackupTrees.execute(activity.contentResolver,invoke.parseArgs(BackupTreeArgs::class.java))) }
      catch (_: Exception) { invoke.reject("backup folder unavailable") }
    }
  }

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
