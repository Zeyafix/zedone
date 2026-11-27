package app.zedone.local

import android.content.ContentResolver
import android.net.Uri
import android.provider.DocumentsContract as Docs
import app.tauri.annotation.InvokeArg
import app.tauri.plugin.JSObject
import org.json.JSONArray

@InvokeArg
class BackupFolderArgs { var writable: Boolean = false }

@InvokeArg
class BackupTreeArgs {
  var root: String = ""
  var path: List<String> = emptyList()
  var operation: String = ""
  var bytes: List<Int>? = null
  var limit: Int = 0
  var offset: Long = 0
}

object BackupTrees {
  private val projection = arrayOf(Docs.Document.COLUMN_DOCUMENT_ID, Docs.Document.COLUMN_DISPLAY_NAME, Docs.Document.COLUMN_MIME_TYPE)
  private fun children(resolver: ContentResolver, tree: Uri, id: String): List<Triple<String,String,Boolean>> {
    val result = ArrayList<Triple<String,String,Boolean>>()
    resolver.query(Docs.buildChildDocumentsUriUsingTree(tree,id),projection,null,null,null)?.use { cursor ->
      while (cursor.moveToNext()) {
        require(result.size < 10000)
        result.add(Triple(cursor.getString(0),cursor.getString(1),cursor.getString(2)==Docs.Document.MIME_TYPE_DIR))
      }
    } ?: throw java.io.IOException()
    require(result.map { it.second }.distinct().size==result.size)
    return result
  }
  fun execute(resolver: ContentResolver, args: BackupTreeArgs): JSObject {
    require(args.root.length in 2..16384 && args.root.length % 2 == 0)
    val tree=Uri.parse(String(ByteArray(args.root.length/2){args.root.substring(it*2,it*2+2).toInt(16).toByte()},Charsets.UTF_8))
    require(tree.scheme=="content" && tree.authority in setOf("com.android.externalstorage.documents","com.android.providers.downloads.documents"))
    require(args.path.size <= 2 && args.path.all { it.length in 1..180 && it!="." && it!=".." && it.all { c -> c.isLetterOrDigit() && c.code<128 || c in "-._" } })
    var id=Docs.getTreeDocumentId(tree)
    var directory=true
    var exists=true
    args.path.forEachIndexed { index,name ->
      require(directory)
      val child=children(resolver,tree,id).singleOrNull { it.second==name }
      if (child==null) {
        require(index==args.path.lastIndex)
        exists=false
      } else { id=child.first;directory=child.third }
    }
    val reply=JSObject().put("entries",JSONArray()).put("bytes",JSONArray()).put("exists",exists)
    val uri=Docs.buildDocumentUriUsingTree(tree,id)
    when(args.operation) {
      "exists" -> Unit
      "directory" -> {
        require(exists && directory)
        resolver.query(uri,projection,null,null,null)?.use { cursor -> require(cursor.moveToFirst() && cursor.getString(2)==Docs.Document.MIME_TYPE_DIR) } ?: throw java.io.IOException()
      }
      "label" -> {
        require(exists && directory)
        val document=Docs.getTreeDocumentId(tree)
        val label=if(document.contains(":")) document.substringAfter(":").ifEmpty { "Память устройства" } else {
          resolver.query(uri,projection,null,null,null)?.use { cursor -> require(cursor.moveToFirst());cursor.getString(1) } ?: throw java.io.IOException()
        }
        reply.put("bytes",JSONArray(label.toByteArray(Charsets.UTF_8).map { it.toInt() and 255 }))
      }
      "list" -> {
        require(exists && directory)
        val entries=JSONArray()
        children(resolver,tree,id).forEach { entries.put(JSObject().put("name",it.second).put("directory",it.third)) }
        reply.put("entries",entries)
      }
      "read", "read_chunk" -> {
        require(exists && !directory && args.limit in 1..67108864)
        val bytes=resolver.openInputStream(uri)?.use { input ->
          if(args.operation=="read_chunk") {
            require(args.offset>=0 && args.limit==65536)
            var skip=args.offset
            while(skip>0) {val count=input.skip(skip);if(count==0L) {require(input.read()>=0);skip--} else skip-=count}
          }
          val output=java.io.ByteArrayOutputStream()
          val buffer=ByteArray(8192)
          val boundary=args.limit+if(args.operation=="read") 1 else 0
          while(output.size() < boundary) {
            val count=input.read(buffer,0,minOf(buffer.size,boundary-output.size()))
            if(count<0) break
            require(count>0)
            output.write(buffer,0,count)
          }
          require(output.size()<=args.limit)
          output.toByteArray()
        } ?: throw java.io.IOException()
        try { reply.put("bytes",JSONArray(bytes.map { it.toInt() and 255 })) } finally { bytes.fill(0) }
      }
      "append" -> {
        require(exists && !directory && args.offset>=0)
        val bytes=args.bytes ?: throw java.io.IOException()
        require(bytes.size<=65536 && bytes.all { it in 0..255 })
        resolver.openFileDescriptor(uri,"wa")?.use { descriptor ->
          java.io.FileOutputStream(descriptor.fileDescriptor).use { output ->
            require(output.channel.size()==args.offset)
            output.write(ByteArray(bytes.size){bytes[it].toByte()});output.flush();output.fd.sync()
          }
        } ?: throw java.io.IOException()
      }
      "mkdir", "create", "write" -> {
        require(args.path.isNotEmpty())
        val writing=args.operation!="mkdir"
        if(args.operation!="write") require(!exists)
        if(exists) require(!directory)
        val target=if(exists) uri else Docs.createDocument(resolver,uri,if(writing) "application/octet-stream" else Docs.Document.MIME_TYPE_DIR,args.path.last()) ?: throw java.io.IOException()
        resolver.query(target,projection,null,null,null)?.use { cursor -> require(cursor.moveToFirst() && cursor.getString(1)==args.path.last()) } ?: throw java.io.IOException()
        if(writing) {
          val input=args.bytes ?: throw java.io.IOException()
          require(input.size<=67108864 && input.all { it in 0..255 })
          val bytes=ByteArray(input.size){input[it].toByte()}
          try {
            resolver.openFileDescriptor(target,"rwt")?.use { descriptor ->
              java.io.FileOutputStream(descriptor.fileDescriptor).use { it.write(bytes);it.flush();it.fd.sync() }
            } ?: throw java.io.IOException()
          } finally { bytes.fill(0) }
        }
      }
      "remove_file", "remove_dir" -> {
        require(exists && args.path.isNotEmpty())
        require(directory==(args.operation=="remove_dir"))
        if(directory) require(children(resolver,tree,id).isEmpty())
        require(Docs.deleteDocument(resolver,uri))
      }
      "sync" -> require(exists && directory)
      else -> throw IllegalArgumentException()
    }
    return reply
  }
}
