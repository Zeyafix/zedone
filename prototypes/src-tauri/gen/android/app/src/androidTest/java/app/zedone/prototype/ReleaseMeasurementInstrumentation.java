package app.zedone.prototype;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.net.LocalServerSocket;
import android.net.LocalSocket;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.WebView;
import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import org.json.JSONObject;
import org.json.JSONTokener;

/** Test-APK-only adapter for the existing JS scenario. Uses only framework/Java
 * APIs: application R8 can remove AndroidX/Kotlin runtime APIs used only by tests.
 * No driver or debugging interface is added to the application itself. */
public class ReleaseMeasurementInstrumentation extends Instrumentation {
  private Bundle arguments;
  private Activity activity;

  @Override public void onCreate(Bundle arguments) {
    super.onCreate(arguments);
    this.arguments = arguments;
    start();
  }

  private WebView findWebView(View view) {
    if (view instanceof WebView) return (WebView) view;
    if (view instanceof ViewGroup) {
      ViewGroup group = (ViewGroup) view;
      for (int i = 0; i < group.getChildCount(); i++) {
        WebView found = findWebView(group.getChildAt(i));
        if (found != null) return found;
      }
    }
    return null;
  }

  private String evaluate(String script) throws Exception {
    CountDownLatch latch = new CountDownLatch(1);
    String[] result = { "null" };
    runOnMainSync(() -> {
      WebView view = findWebView(activity.getWindow().getDecorView());
      if (view == null) throw new IllegalStateException("Measured WebView unavailable");
      view.evaluateJavascript(script, value -> { result[0] = value; latch.countDown(); });
    });
    if (!latch.await(5, TimeUnit.SECONDS)) throw new IllegalStateException("WebView evaluation timed out");
    return result[0];
  }

  @Override public void onStart() {
    Bundle result = new Bundle();
    try {
      drive();
      result.putString("stream", "Release measurement driver completed\n");
      finish(Activity.RESULT_OK, result);
    } catch (Throwable error) {
      result.putString("stream", android.util.Log.getStackTraceString(error));
      finish(Activity.RESULT_CANCELED, result);
    }
  }

  private void drive() throws Exception {
    if ((getTargetContext().getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) != 0)
      throw new IllegalStateException("Release application is debuggable");
    String name = arguments.getString("measurementSocket", "");
    if (!name.matches("zedone-measure-[a-f0-9]{32}")) throw new IllegalArgumentException("Invalid socket name");
    // The runner owns final teardown of Tauri's last window.
    activity = startActivitySync(new Intent(Intent.ACTION_MAIN)
      .setClassName(getTargetContext().getPackageName(), getTargetContext().getPackageName() + ".MainActivity")
      .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
    long readyDeadline = SystemClock.elapsedRealtime() + 20000;
    while (!evaluate("!!window.__TAURI_INTERNALS__ && !!document.querySelector('#board-read')").equals("true")) {
      if (SystemClock.elapsedRealtime() >= readyDeadline) throw new IllegalStateException("Release WebView not ready");
      SystemClock.sleep(20);
    }
    try (LocalServerSocket server = new LocalServerSocket(name); LocalSocket socket = server.accept()) {
      // ADB forwarding is served by adbd: root on AOSP, shell on retail Android.
      int uid = socket.getPeerCredentials().getUid();
      if (uid != 0 && uid != 2000) throw new SecurityException("Only an ADB driver may connect");
      BufferedReader reader = new BufferedReader(new InputStreamReader(socket.getInputStream(), StandardCharsets.UTF_8));
      BufferedWriter writer = new BufferedWriter(new OutputStreamWriter(socket.getOutputStream(), StandardCharsets.UTF_8));
      String line;
      while ((line = reader.readLine()) != null) {
        JSONObject request = new JSONObject(line);
        if (request.optBoolean("close")) break;
        String script = request.getString("script");
        if (script.length() > 1000000) throw new IllegalArgumentException("Command too large");
        evaluate("window.__releaseMeasurementResult = null; (async () => { try { window.__releaseMeasurementResult = {ok:true,value:await (" + script + ")}; } catch(e) { window.__releaseMeasurementResult = {ok:false,error:String(e.stack || e)}; } })(); true");
        long deadline = SystemClock.elapsedRealtime() + 12000;
        String response;
        do {
          // evaluateJavascript returns a JSON-encoded string.
          response = new JSONTokener(evaluate("JSON.stringify(window.__releaseMeasurementResult)")).nextValue().toString();
          if (!response.equals("null")) break;
          if (SystemClock.elapsedRealtime() >= deadline) throw new IllegalStateException("Measurement command timed out");
          SystemClock.sleep(5);
        } while (true);
        writer.write(response); writer.newLine(); writer.flush();
        evaluate("delete window.__releaseMeasurementResult; true");
      }
    }
  }
}
