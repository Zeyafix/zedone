package app.zedone.prototype;

import android.app.Activity;
import android.app.Instrumentation;
import android.app.UiAutomation;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.graphics.Bitmap;
import android.net.LocalServerSocket;
import android.net.LocalSocket;
import android.os.Bundle;
import android.os.SystemClock;
import android.os.Trace;
import android.util.Base64;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.WebView;
import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.ByteArrayOutputStream;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.zip.Deflater;
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

  private static class Capture {
    final JSONObject metadata;
    final byte[] rgba;
    Capture(JSONObject metadata, byte[] rgba) { this.metadata = metadata; this.rgba = rgba; }
  }

  // Both transports read the same OS display bitmap. No renderer screenshot,
  // downsampling or tolerance; the host clock includes extraction and transfer.
  private Capture screenshot(JSONObject request, boolean tracing) throws Exception {
    JSONObject region = request.optJSONObject("region");
    String format = request.optString("format", "png");
    if (!format.equals("png") && !format.equals("rgba8") && !format.equals("rgba8-zlib"))
      throw new IllegalArgumentException("Unknown screenshot format");
    boolean raw = !format.equals("png");
    long start = SystemClock.elapsedRealtimeNanos();
    Bitmap screen;
    if (tracing) Trace.beginSection("zedone:OS-readback");
    try {
      screen = getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES).takeScreenshot();
    } finally { if (tracing) Trace.endSection(); }
    if (screen == null) throw new IllegalStateException("OS screenshot unavailable");
    Bitmap cropped = null;
    try {
      int x = region == null ? 0 : region.getInt("x");
      int y = region == null ? 0 : region.getInt("y");
      int width = region == null ? screen.getWidth() : region.getInt("width");
      int height = region == null ? screen.getHeight() : region.getInt("height");
      if (x < 0 || y < 0 || width <= 0 || height <= 0 ||
          (long) x + width > screen.getWidth() || (long) y + height > screen.getHeight())
        throw new IllegalArgumentException("Capture region outside Android image");
      if (tracing) Trace.beginSection(raw ? "zedone:extract-RGBA" : "zedone:crop-PNG-base64");
      try {
      JSONObject metadata = new JSONObject().put("format", format)
        .put("screen", new JSONObject().put("width", screen.getWidth()).put("height", screen.getHeight()));
      byte[] rgba = null;
      if (raw) {
        long length = (long) width * height * 4;
        if (length > 64 * 1024 * 1024) throw new IllegalArgumentException("RGBA image too large");
        int[] colors = new int[width * height];
        // getPixels returns non-premultiplied sRGB ARGB ints, independent of
        // native bitmap stride/byte order. Send explicit packed RGBA rows.
        screen.getPixels(colors, 0, width, x, y, width, height);
        rgba = new byte[(int) length];
        for (int i = 0, j = 0; i < colors.length; i++) {
          int color = colors[i];
          rgba[j++] = (byte) (color >>> 16);
          rgba[j++] = (byte) (color >>> 8);
          rgba[j++] = (byte) color;
          rgba[j++] = (byte) (color >>> 24);
        }
        metadata.put("width", width).put("height", height).put("byte_length", rgba.length);
        if (format.equals("rgba8-zlib")) {
          long compressionStart = SystemClock.elapsedRealtimeNanos();
          if (tracing) Trace.beginSection("zedone:deflate-RGBA");
          // A complete zlib stream per bitmap; no inter-frame dictionary.
          Deflater compressor = new Deflater(Deflater.BEST_SPEED);
          try {
            compressor.setInput(rgba);
            compressor.finish();
            ByteArrayOutputStream compressed = new ByteArrayOutputStream();
            byte[] chunk = new byte[16384];
            while (!compressor.finished()) {
              int count = compressor.deflate(chunk);
              if (count <= 0) throw new IllegalStateException("RGBA deflater stalled");
              compressed.write(chunk, 0, count);
              if (compressed.size() > 65 * 1024 * 1024)
                throw new IllegalStateException("Compressed RGBA image too large");
            }
            rgba = compressed.toByteArray();
          } finally {
            compressor.end();
            if (tracing) Trace.endSection();
          }
          metadata.put("wire_byte_length", rgba.length)
            .put("compression_ms", (SystemClock.elapsedRealtimeNanos() - compressionStart) / 1000000.0);
        }
      }
      // verify_png is only used outside intervals: compare both encodings of
      // this exact bitmap as well as independent, stable ADB PNG captures.
      if (!raw || request.optBoolean("verify_png")) {
      cropped = Bitmap.createBitmap(screen, x, y, width, height);
      ByteArrayOutputStream bytes = new ByteArrayOutputStream();
      if (!cropped.compress(Bitmap.CompressFormat.PNG, 100, bytes))
        throw new IllegalStateException("OS screenshot encoding failed");
      metadata.put("png", Base64.encodeToString(bytes.toByteArray(), Base64.NO_WRAP));
      }
      metadata.put("capture_and_encode_ms", (SystemClock.elapsedRealtimeNanos() - start) / 1000000.0);
      return new Capture(metadata, rgba);
      } finally { if (tracing) Trace.endSection(); }
    } finally {
      if (cropped != null && cropped != screen) cropped.recycle();
      screen.recycle();
    }
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
        String label = request.optString("trace_label", "");
        if (!label.isEmpty() && !label.matches("zedone:[a-z0-9-]{1,80}"))
          throw new IllegalArgumentException("Invalid trace label");
        boolean tracing = !label.isEmpty();
        if (tracing) Trace.beginSection(label + (request.optBoolean("screenshot") ? ":capture" : ":evaluate"));
        try {
        if (request.optBoolean("screenshot")) {
          Capture capture = screenshot(request, tracing);
          writer.write(new JSONObject().put("ok", true).put("value", capture.metadata).toString());
          writer.newLine(); writer.flush();
          if (capture.rgba != null) {
            socket.getOutputStream().write(capture.rgba);
            socket.getOutputStream().flush();
          }
          continue;
        }
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
        } finally { if (tracing) Trace.endSection(); }
      }
    }
  }
}
