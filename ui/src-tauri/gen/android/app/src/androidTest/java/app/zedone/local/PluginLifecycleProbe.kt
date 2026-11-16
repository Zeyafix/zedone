package app.zedone.local

import android.app.Activity
import android.app.Instrumentation
import android.content.Intent
import android.os.SystemClock
import android.webkit.WebView
import app.tauri.plugin.Plugin
import app.tauri.plugin.PluginHandle
import app.tauri.plugin.PluginManager

fun verifyPluginLifecycle(instrumentation: Instrumentation) {
  fun <T> main(work: () -> T): T {
    var result: Result<T>? = null
    instrumentation.runOnMainSync { result = runCatching(work) }
    return result!!.getOrThrow()
  }
  val field = PluginManager.javaClass.getDeclaredField("plugins").apply { isAccessible = true }
  val names = listOf("app", "path", "opener", "barcode-scanner", "host-secrets")
  fun ready(activity: Activity) {
    val deadline = SystemClock.uptimeMillis() + 10000
    while (SystemClock.uptimeMillis() < deadline) {
      if (main {
        @Suppress("UNCHECKED_CAST")
        val handles = field.get(PluginManager) as Map<String, PluginHandle>
        PluginManager.activity === activity && names.all { handles[it]?.loaded == true }
      }) return
      SystemClock.sleep(25)
    }
    error("Native plugins did not bind to the current Activity/WebView")
  }
  val initial = instrumentation.startActivitySync(Intent(instrumentation.targetContext, MainActivity::class.java).apply {
    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
  })
  ready(initial)
  val monitor = instrumentation.addMonitor(MainActivity::class.java.name, null, false)
  val current: Activity
  try {
    main { initial.recreate() }
    current = monitor.waitForActivityWithTimeout(10000) ?: error("Recreated Activity unavailable")
  } finally { instrumentation.removeMonitor(monitor) }
  ready(current)
  main {
    check(initial !== current && initial.isDestroyed)
    @Suppress("UNCHECKED_CAST")
    val handles = field.get(PluginManager) as Map<String, PluginHandle>
    val owner = Plugin::class.java.getDeclaredField("activity").apply { isAccessible = true }
    for (name in names) {
      val instance = handles.getValue(name).instance
      check(owner.get(instance) === current) { "$name retained a destroyed Activity" }
      val viewField = instance.javaClass.declaredFields.firstOrNull { it.name == "webView" }
      if (viewField != null) {
        viewField.isAccessible = true
        check((viewField.get(instance) as WebView).context === current) { "$name retained an old WebView" }
      }
    }
  }
}
