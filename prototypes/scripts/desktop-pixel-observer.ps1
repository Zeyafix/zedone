$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$observerSource = @'
using System;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Text;
public static class ScreenObserver {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int count);
  delegate bool WindowCallback(IntPtr hwnd, IntPtr parameter);
  [DllImport("user32.dll")] static extern bool EnumWindows(WindowCallback callback, IntPtr parameter);
  [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr hwnd, int command);
  [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; public POINT(int x,int y){X=x;Y=y;} }
  [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(POINT point);
  [DllImport("user32.dll")] static extern IntPtr GetAncestor(IntPtr hwnd,uint flags);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
  [DllImport("user32.dll")] static extern bool IsIconic(IntPtr hwnd);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] static extern IntPtr GetWindowLongPtr(IntPtr hwnd,int index);
  [DllImport("user32.dll")] static extern IntPtr GetThreadDpiAwarenessContext();
  [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
  public static void Init() { SetThreadDpiAwarenessContext(new IntPtr(-4)); }
  public static object Expose(string prefix) {
    if(!prefix.StartsWith("ZeDone "))throw new Exception("Only isolated ZeDone test windows may be exposed");
    IntPtr found=IntPtr.Zero;
    EnumWindows(delegate(IntPtr hwnd,IntPtr p){var title=new StringBuilder(1024);GetWindowText(hwnd,title,title.Capacity);if(title.ToString().StartsWith(prefix)){found=hwnd;return false;}return true;},IntPtr.Zero);
    if(found==IntPtr.Zero)throw new Exception("Isolated test window not found on this desktop");
    ShowWindow(found,4);
    if(!SetWindowPos(found,new IntPtr(-1),0,0,0,0,0x53))throw new Exception("Cannot expose isolated test window");
    Thread.Sleep(100);
    return Describe(found);
  }
  public static object Window() {
    return Describe(GetForegroundWindow());
  }
  static object Describe(IntPtr hwnd) {
    RECT r; uint pid;
    if(hwnd==IntPtr.Zero || !GetWindowRect(hwnd,out r)) throw new Exception("No foreground window");
    GetWindowThreadProcessId(hwnd,out pid);
    var title=new StringBuilder(1024); GetWindowText(hwnd,title,title.Capacity);
    return new { hwnd=hwnd.ToInt64(), pid=pid, process=Process.GetProcessById((int)pid).ProcessName, title=title.ToString(), x=r.Left, y=r.Top, width=r.Right-r.Left, height=r.Bottom-r.Top, frequency=Stopwatch.Frequency, visible=IsWindowVisible(hwnd), iconic=IsIconic(hwnd), ex_style=GetWindowLongPtr(hwnd,-20).ToInt64(), dpi_context=GetThreadDpiAwarenessContext().ToInt64() };
  }
  static void Check(IntPtr hwnd, int x, int y, int width, int height) {
    RECT r;
    if(hwnd==IntPtr.Zero || IsIconic(hwnd)) throw new Exception("Test window is unavailable or minimized");
    if(!GetWindowRect(hwnd,out r) || x<r.Left || y<r.Top || x+width>r.Right || y+height>r.Bottom) throw new Exception("Capture region is outside the test window");
    if(width<1 || height<1 || width*height>1000000) throw new Exception("Invalid or excessive capture region");
    foreach(var point in new POINT[]{new POINT(x,y),new POINT(x+width-1,y),new POINT(x,y+height-1),new POINT(x+width-1,y+height-1),new POINT(x+width/2,y+height/2)}) {
      var top=GetAncestor(WindowFromPoint(point),2);
      if(top!=hwnd){uint pid;GetWindowThreadProcessId(top,out pid);throw new Exception("Test capture region is occluded; point="+point.X+","+point.Y+"; expected_window="+hwnd.ToInt64()+"; actual_window="+top.ToInt64()+"; actual_pid="+pid);}
    }
  }
  static byte[] Pixels(int x,int y,int width,int height,out byte[] png) {
    using(var bmp=new Bitmap(width,height,PixelFormat.Format32bppArgb)) {
      using(var g=Graphics.FromImage(bmp)) g.CopyFromScreen(x,y,0,0,new Size(width,height),CopyPixelOperation.SourceCopy);
      var data=bmp.LockBits(new Rectangle(0,0,width,height),ImageLockMode.ReadOnly,PixelFormat.Format32bppArgb);
      var bytes=new byte[width*height*4];
      for(int row=0;row<height;row++) Marshal.Copy(IntPtr.Add(data.Scan0,row*data.Stride),bytes,row*width*4,width*4);
      bmp.UnlockBits(data);
      for(int i=3;i<bytes.Length;i+=4) bytes[i]=255;
      using(var stream=new MemoryStream()) { bmp.Save(stream,ImageFormat.Png); png=stream.ToArray(); }
      return bytes;
    }
  }
  static bool Equal(byte[] a,byte[] b) { if(a.Length!=b.Length)return false; for(int i=0;i<a.Length;i++)if(a[i]!=b[i])return false; return true; }
  public static object Capture(long window,int x,int y,int width,int height) {
    var hwnd=new IntPtr(window); Check(hwnd,x,y,width,height); byte[] png;
    var start=Stopwatch.GetTimestamp(); var pixels=Pixels(x,y,width,height,out png); var end=Stopwatch.GetTimestamp();
    return new { pixels=Convert.ToBase64String(pixels), png=Convert.ToBase64String(png), capture_ms=(end-start)*1000.0/Stopwatch.Frequency };
  }
  public static object Observe(long window,int x,int y,int width,int height,string expected,int timeoutMs) {
    var hwnd=new IntPtr(window);Check(hwnd,x,y,width,height);byte[] png;
    var target=Convert.FromBase64String(expected);var baseline=Pixels(x,y,width,height,out png);
    if(target.Length!=width*height*4 || timeoutMs<1 || timeoutMs>12000)throw new Exception("Invalid pixel template or timeout");
    if(Equal(baseline,target))throw new Exception("Expected result is already visible before command");
    var start=Stopwatch.GetTimestamp();
    Console.WriteLine("{\"armed\":true}");Console.Out.Flush();
    var lastMiss=start;int captures=0;double maxCapture=0;
    while((Stopwatch.GetTimestamp()-start)*1000.0/Stopwatch.Frequency<timeoutMs) {
      Check(hwnd,x,y,width,height);
      var captureStart=Stopwatch.GetTimestamp();var pixels=Pixels(x,y,width,height,out png);var captureEnd=Stopwatch.GetTimestamp();captures++;
      maxCapture=Math.Max(maxCapture,(captureEnd-captureStart)*1000.0/Stopwatch.Frequency);
      if(Equal(pixels,target))return new{ok=true,arm_to_pixels_lower_ms=(lastMiss-start)*1000.0/Stopwatch.Frequency,
        upper_bound_ms=(captureEnd-start)*1000.0/Stopwatch.Frequency,captures=captures,max_capture_ms=maxCapture,
        png=Convert.ToBase64String(png),clock="Windows QPC / Stopwatch",boundary="observer armed before DOM command -> matching desktop pixels; conservative upper bound includes driver/CDP dispatch"};
      lastMiss=captureStart;Thread.Sleep(1);
    }
    return new{ok=false,error="Expected desktop pixels not observed before timeout",elapsed_ms=(Stopwatch.GetTimestamp()-start)*1000.0/Stopwatch.Frequency,captures=captures,max_capture_ms=maxCapture};
  }
}
'@
Add-Type -TypeDefinition $observerSource -ReferencedAssemblies System.Drawing
[ScreenObserver]::Init()
Write-Output '{"ready":true}'
while ($null -ne ($observerLine = [Console]::ReadLine())) {
  try {
    $observerRequest = $observerLine | ConvertFrom-Json
    $observerResult = switch ($observerRequest.action) {
      'window' { [ScreenObserver]::Window() }
      'expose' { [ScreenObserver]::Expose($observerRequest.prefix) }
      'capture' { [ScreenObserver]::Capture($observerRequest.hwnd,$observerRequest.x,$observerRequest.y,$observerRequest.width,$observerRequest.height) }
      'observe' { [ScreenObserver]::Observe($observerRequest.hwnd,$observerRequest.x,$observerRequest.y,$observerRequest.width,$observerRequest.height,$observerRequest.expected,$observerRequest.timeout_ms) }
      default { throw 'Unknown observer action' }
    }
    $observerResult | ConvertTo-Json -Compress -Depth 6 | Write-Output
  } catch {
    @{ok=$false; error=$_.Exception.Message} | ConvertTo-Json -Compress | Write-Output
  }
}
