package app.zedone.local

import android.app.Activity
import android.app.Instrumentation
import android.content.Intent
import android.os.Bundle
import android.provider.DocumentsContract
import java.util.UUID

fun verifySelectedBackupFolder(instrumentation: Instrumentation) {
  val result=Bundle()
  var success=false
  try {
    val resolver=instrumentation.targetContext.contentResolver
    val permission=resolver.persistedUriPermissions.single { permission ->
      permission.uri.authority in setOf("com.android.externalstorage.documents","com.android.providers.downloads.documents") && DocumentsContract.getTreeDocumentId(permission.uri).endsWith("/ZeDone-P0705")
    }
    val root=permission.uri.toString().toByteArray(Charsets.UTF_8).joinToString("") { "%02x".format(it.toInt() and 255) }
    fun execute(operation:String,path:List<String> = emptyList(),bytes:List<Int>? = null,offset:Long=0,limit:Int=0)=BackupTrees.execute(resolver,BackupTreeArgs().apply {this.root=root;this.operation=operation;this.path=path;this.bytes=bytes;this.offset=offset;this.limit=limit})
    fun denied(work:()->Unit) {check(runCatching(work).isFailure)}
    execute("directory")
    check(execute("list").getJSONArray("entries").length()==0)
    val directory="zedone-probe-"+UUID.randomUUID().toString()
    try {
      execute("mkdir",listOf(directory))
      execute("create",listOf(directory,"probe.bin"),emptyList())
      execute("append",listOf(directory,"probe.bin"),listOf(0,1,2),0)
      denied {execute("append",listOf(directory,"probe.bin"),listOf(3),0)}
      check(execute("read",listOf(directory,"probe.bin"),limit=3).getJSONArray("bytes").toString()=="[0,1,2]")
      check(execute("read_chunk",listOf(directory,"probe.bin"),offset=1,limit=65536).getJSONArray("bytes").toString()=="[1,2]")
      denied {execute("read",listOf(directory,"probe.bin"),limit=2)}
      denied {execute("create",listOf(directory,"probe.bin"),listOf(9))}
      denied {execute("remove_dir",listOf(directory))}
      denied {execute("read",listOf("..","probe.bin"),limit=3)}
      denied {execute("read",listOf(directory,"nested","probe.bin"),limit=3)}
    } finally {
      execute("remove_file",listOf(directory,"probe.bin"))
      execute("remove_dir",listOf(directory))
    }
    check(execute("list").getJSONArray("entries").length()==0)
    val flags=(if(permission.isReadPermission) Intent.FLAG_GRANT_READ_URI_PERMISSION else 0) or (if(permission.isWritePermission) Intent.FLAG_GRANT_WRITE_URI_PERMISSION else 0)
    resolver.releasePersistableUriPermission(permission.uri,flags)
    denied {execute("directory")}
    check(resolver.persistedUriPermissions.none { it.uri==permission.uri })
    result.putString("zedone_backup_folder","passed")
    result.putString("zedone_backup_folder_permission","released")
    success=true
  } catch (error:Throwable) {
    result.putString("zedone_backup_folder","failed")
    result.putString("failure_type",error.javaClass.simpleName)
  }
  instrumentation.finish(if(success) Activity.RESULT_OK else Activity.RESULT_CANCELED,result)
}
