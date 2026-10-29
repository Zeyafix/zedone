package app.zedone.prototype

import android.app.Activity
import android.webkit.WebView
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.tauri.plugin.Plugin
import app.tauri.plugin.PluginHandle
import app.tauri.plugin.PluginLifecycle
import app.tauri.plugin.PluginManager
import com.fasterxml.jackson.databind.ObjectMapper
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

class ProbePlugin(val owner: Activity) : Plugin(owner) {
  var view: WebView? = null
  var destroys = 0
  override fun load(webView: WebView) { view = webView }
  override fun onDestroy(activity: androidx.appcompat.app.AppCompatActivity) { destroys++ }
}

@RunWith(AndroidJUnit4::class)
class PluginLifecycleTest {
  private fun awaitPlugins(scenario: ActivityScenario<MainActivity>) {
    val field = PluginManager.javaClass.getDeclaredField("plugins").apply { isAccessible = true }
    repeat(200) {
      InstrumentationRegistry.getInstrumentation().waitForIdleSync()
      var ready = false
      scenario.onActivity {
        @Suppress("UNCHECKED_CAST")
        val map = field.get(PluginManager) as Map<String, PluginHandle>
        ready = listOf("app", "path", "opener", "barcode-scanner").all { map[it]?.loaded == true }
      }
      if (ready) return
      android.os.SystemClock.sleep(50)
    }
    error("Native plugins did not bind to the WebView")
  }

  // This is intentionally a real framework recreation, not a mocked lifecycle.
  @Test fun recreatesNativeOwnersAndPreservesConfiguration() {
    val handles = linkedMapOf<String, PluginHandle>()
    val lifecycle = PluginLifecycle(PluginManager, handles)
    var original: ProbePlugin? = null
    var oldActivity: MainActivity? = null
    val config = "{\"synthetic\":\"Ёж\"}"
    // Closing Tauri's final window exits the native process before JUnit can
    // report its result. The instrumentation runner owns final process teardown.
    val scenario = ActivityScenario.launch(MainActivity::class.java)
    run {
      awaitPlugins(scenario)
      scenario.onActivity { activity ->
        oldActivity = activity
        lifecycle.onActivityAvailable(activity)
        val view = WebView(activity)
        lifecycle.onWebViewCreated(activity, view)
        val probe = ProbePlugin(activity)
        lifecycle.register(activity, view, "probe", probe, config, ObjectMapper())
        original = probe
        assertSame(probe, handles.getValue("probe").instance)
        assertSame(view, probe.view)
        lifecycle.onDestroy(activity)
        assertTrue(handles.isEmpty())
        assertEquals(1, probe.destroys)
        view.destroy()
      }
      scenario.recreate()
      awaitPlugins(scenario)
      scenario.onActivity { activity ->
        assertNotSame(oldActivity, activity)
        assertTrue(oldActivity!!.isDestroyed)
        lifecycle.onActivityAvailable(activity)
        val view = WebView(activity)
        lifecycle.onWebViewCreated(activity, view)
        val handle = handles.getValue("probe")
        val probe = handle.instance as ProbePlugin
        assertNotSame(original, probe)
        assertSame(activity, probe.owner)
        assertSame(view, probe.view)
        assertEquals(config, handle.config)
        assertTrue(handle.loaded)
        // Verify the actual Tauri registrations, not just the isolated probe.
        assertSame(activity, PluginManager.activity)
        val field = PluginManager.javaClass.getDeclaredField("plugins").apply { isAccessible = true }
        @Suppress("UNCHECKED_CAST")
        val actual = field.get(PluginManager) as Map<String, PluginHandle>
        for (name in listOf("app", "path", "opener", "barcode-scanner")) {
          val instance = actual.getValue(name).instance
          val owner = Plugin::class.java.getDeclaredField("activity").apply { isAccessible = true }.get(instance)
          assertSame("$name uses the recreated Activity", activity, owner)
          val viewField = instance.javaClass.declaredFields.firstOrNull { it.name == "webView" }
          if (viewField != null) {
            viewField.isAccessible = true
            val nativeView = viewField.get(instance) as WebView
            assertSame("$name uses a current WebView", activity, nativeView.context)
          }
        }
        lifecycle.onDestroy(activity)
        assertEquals(1, probe.destroys)
        view.destroy()
      }
    }
  }
}
