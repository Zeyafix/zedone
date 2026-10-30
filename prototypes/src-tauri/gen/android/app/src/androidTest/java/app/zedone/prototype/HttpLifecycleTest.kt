package app.zedone.prototype

import android.webkit.WebView
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(AndroidJUnit4::class)
class HttpLifecycleTest {
  private val mapper = ObjectMapper()

  private fun evaluate(scenario: ActivityScenario<MainActivity>, script: String): String {
    val latch = CountDownLatch(1)
    var result = ""
    scenario.onActivity { activity ->
      val field = MainActivity::class.java.getDeclaredField("prototypeWebView").apply { isAccessible = true }
      val view = field.get(activity) as? WebView
      if (view == null) { result = "null"; latch.countDown() }
      else view.evaluateJavascript(script) { result = it; latch.countDown() }
    }
    assertTrue("WebView evaluation completed", latch.await(5, TimeUnit.SECONDS))
    return result
  }

  private fun control(scenario: ActivityScenario<MainActivity>, request: String): JsonNode {
    var ready = false
    for (attempt in 0 until 200) {
      if (evaluate(scenario, "!!window.__TAURI_INTERNALS__") == "true") { ready = true; break }
      android.os.SystemClock.sleep(25)
    }
    assertTrue("IPC bridge available", ready)
    evaluate(scenario, "window.__httpLifecycleResult = ''; window.__TAURI_INTERNALS__.invoke('http_session', {request:$request}).then(r => window.__httpLifecycleResult = JSON.stringify(r)).catch(e => window.__httpLifecycleResult = JSON.stringify({error:String(e)})); true")
    repeat(200) {
      val raw = evaluate(scenario, "window.__httpLifecycleResult || ''")
      val result = mapper.readValue(raw, String::class.java)
      if (result.isNotEmpty()) {
        val value = mapper.readTree(result)
        assertFalse("local session command succeeded", value.has("error"))
        return value
      }
      android.os.SystemClock.sleep(25)
    }
    error("HTTP session command did not complete")
  }

  private fun echo(endpoint: String, token: String): Int {
    val connection = URL("$endpoint/prototype/echo").openConnection() as HttpURLConnection
    connection.requestMethod = "POST"
    connection.setRequestProperty("Authorization", "Bearer $token")
    connection.setRequestProperty("Content-Type", "application/json")
    connection.connectTimeout = 3000
    connection.readTimeout = 3000
    connection.doOutput = true
    try {
      connection.outputStream.use { it.write("{\"text\":\"recreation\"}".toByteArray()) }
      return connection.responseCode
    } finally { connection.disconnect() }
  }

  @Test fun recreationRevokesHttpAndAllowsOnlyAnExplicitNewSession() {
    // The instrumentation runner owns teardown; closing Tauri's last window exits.
    val scenario = ActivityScenario.launch(MainActivity::class.java)
    control(scenario, "{\"action\":\"stop\"}")
    val first = control(scenario, "{\"action\":\"start\",\"address\":\"127.0.0.1\",\"port\":0}")
    assertEquals(200, echo(first["endpoint"].asText(), first["token"].asText()))
    scenario.recreate()
    assertTrue(control(scenario, "{\"action\":\"status\"}")["endpoint"].isNull)
    val second = control(scenario, "{\"action\":\"start\",\"address\":\"127.0.0.1\",\"port\":0}")
    assertNotEquals(first["token"].asText(), second["token"].asText())
    assertEquals(401, echo(second["endpoint"].asText(), first["token"].asText()))
    assertEquals(200, echo(second["endpoint"].asText(), second["token"].asText()))
    control(scenario, "{\"action\":\"stop\"}")
  }
}
