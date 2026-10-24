package io.andromeda.companion;

import android.accessibilityservice.AccessibilityService;
import android.graphics.Bitmap;
import android.hardware.HardwareBuffer;
import android.os.Build;
import android.os.SystemClock;
import android.util.Base64;
import android.view.Display;
import java.io.ByteArrayOutputStream;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import org.json.JSONObject;

/** One bounded, fresh Android capture. No stream, files, or retained frame cache. */
final class ScreenCapture {
    interface Guard { void check() throws Exception; }
    static final class Failure extends Exception {
        final String code;
        Failure(String code, String message) { super(message); this.code=code; }
    }
    private static int option(JSONObject request,String name,int fallback,int max)throws Failure {
        if(!request.has(name))return fallback;
        Object value=request.opt(name);
        if(!(value instanceof Number number) || number.doubleValue()!=number.longValue() || number.longValue()<1 || number.longValue()>max)throw new Failure("invalid_argument","Invalid capture integer: "+name);
        return number.intValue();
    }
    private static final int MAX_BYTES=3*1024*1024-4096;
    private static final class LimitExceeded extends RuntimeException {}
    private static final class BoundedBytes extends ByteArrayOutputStream {
        boolean exceeded;
        private void reserve(int length) { if(length>MAX_BYTES-count) { exceeded=true;throw new LimitExceeded(); } }
        @Override public synchronized void write(int value) { reserve(1);super.write(value); }
        @Override public synchronized void write(byte[] data,int offset,int length) { reserve(length);super.write(data,offset,length); }
    }
    /** A timeout ends the caller, but only Android's callback ends native work. */
    private static final class CaptureLease {
        final AtomicBoolean gate;
        boolean nativePending,ownerDone,released;
        CaptureLease(AtomicBoolean gate) { this.gate=gate; }
        synchronized void nativeStarted() { nativePending=true; }
        synchronized void nativeFinished() { nativePending=false;releaseIfReady(); }
        synchronized void ownerFinished() { ownerDone=true;releaseIfReady(); }
        private void releaseIfReady() {
            if(ownerDone && !nativePending && !released) { released=true;gate.set(false); }
        }
    }
    private final AtomicBoolean active=new AtomicBoolean();
    JSONObject capture(AccessibilityService service, JSONObject request, Guard guard) throws Exception {
        if(Build.VERSION.SDK_INT<30)throw new Failure("capture_unavailable","Accessibility screenshots require Android 11 or newer");
        return Api30.capture(service,request,guard,active);
    }
    // Keep newer API types out of classes loaded by Android 8–10 services.
    private static final class Api30 {
    @android.annotation.TargetApi(30)
    static JSONObject capture(AccessibilityService service,JSONObject request,Guard guard,AtomicBoolean active)throws Exception {
        String format=request.optString("format","png");
        Bitmap.CompressFormat encoder;
        String mime;
        switch(format) {
            case "png" -> {encoder=Bitmap.CompressFormat.PNG;mime="image/png";}
            case "jpeg" -> {encoder=Bitmap.CompressFormat.JPEG;mime="image/jpeg";}
            case "webp" -> {encoder=Bitmap.CompressFormat.WEBP_LOSSY;mime="image/webp";}
            case "webp_lossless" -> {encoder=Bitmap.CompressFormat.WEBP_LOSSLESS;mime="image/webp";}
            default -> throw new Failure("invalid_argument","Unknown capture image format");
        }
        int maxWidth=option(request,"max_width",8192,8192),maxHeight=option(request,"max_height",8192,8192),quality=option(request,"quality",80,100);
        if(maxWidth<1 || maxWidth>8192 || maxHeight<1 || maxHeight>8192 || quality<1 || quality>100)throw new Failure("invalid_argument","Invalid capture dimensions or quality");
        if(!active.compareAndSet(false,true))throw new Failure("capture_busy","Another accessibility capture is running");
        CaptureLease lease=new CaptureLease(active);
        CompletableFuture<AccessibilityService.ScreenshotResult> pending=new CompletableFuture<>();
        AccessibilityService.ScreenshotResult result=null;
        Bitmap hardware=null,software=null,resized=null;
        long started=SystemClock.uptimeMillis();
        try {
            guard.check();
            int testDelay=BuildConfig.DEBUG?option(request,"test_callback_delay_ms",0,3000):0;
            java.util.concurrent.Executor executor=testDelay==0?Runnable::run:callback->new android.os.Handler(service.getMainLooper()).postDelayed(callback,testDelay);
            lease.nativeStarted();
            try { service.takeScreenshot(Display.DEFAULT_DISPLAY,executor,new AccessibilityService.TakeScreenshotCallback(){
                @Override public void onSuccess(AccessibilityService.ScreenshotResult value) {
                    try { synchronized(pending) { if(!pending.complete(value))value.getHardwareBuffer().close(); } }
                    finally { lease.nativeFinished(); }
                }
                @Override public void onFailure(int error) {
                    String code=error==AccessibilityService.ERROR_TAKE_SCREENSHOT_INTERVAL_TIME_SHORT?"capture_throttled":error==AccessibilityService.ERROR_TAKE_SCREENSHOT_SECURE_WINDOW?"capture_protected":error==AccessibilityService.ERROR_TAKE_SCREENSHOT_NO_ACCESSIBILITY_ACCESS?"capture_permission":"capture_failed";
                    try { pending.completeExceptionally(new Failure(code,"Android screenshot failed (code "+error+")")); }
                    finally { lease.nativeFinished(); }
                }
            }); } catch(RuntimeException | Error failure) { lease.nativeFinished();throw failure; }
            try { result=pending.get(1500,TimeUnit.MILLISECONDS); }
            catch(ExecutionException failure) { if(failure.getCause() instanceof Exception error)throw error;throw failure; }
            long callbackAt=SystemClock.uptimeMillis();
            guard.check();
            HardwareBuffer buffer=result.getHardwareBuffer();
            int nativeWidth=buffer.getWidth(),nativeHeight=buffer.getHeight();
            if(nativeWidth<1 || nativeHeight<1 || nativeWidth>8192 || nativeHeight>8192 || (long)nativeWidth*nativeHeight>8_388_608)throw new Failure("output_limit","Native screenshot exceeds capture limits");
            hardware=Bitmap.wrapHardwareBuffer(buffer,result.getColorSpace());
            if(hardware==null)throw new Failure("capture_unavailable","Android could not wrap the screenshot buffer");
            software=hardware.copy(Bitmap.Config.ARGB_8888,false);
            if(software==null)throw new Failure("capture_unavailable","Android could not copy screenshot pixels");
            double ratio=Math.min(1.0,Math.min((double)maxWidth/nativeWidth,(double)maxHeight/nativeHeight));
            int width=Math.max(1,(int)Math.floor(nativeWidth*ratio)),height=Math.max(1,(int)Math.floor(nativeHeight*ratio));
            resized=(width==nativeWidth && height==nativeHeight)?software:Bitmap.createScaledBitmap(software,width,height,true);
            guard.check();
            BoundedBytes bytes=new BoundedBytes();
            boolean encoded;
            try { encoded=resized.compress(encoder,encoder==Bitmap.CompressFormat.WEBP_LOSSLESS?80:quality,bytes); }
            catch(LimitExceeded limit) { throw new Failure("output_limit","Encoded screenshot exceeds companion response limit"); }
            if(bytes.exceeded)throw new Failure("output_limit","Encoded screenshot exceeds companion response limit");
            if(!encoded)throw new Failure("capture_encoding","Android screenshot encoding failed");
            guard.check();
            if(bytes.size()>MAX_BYTES)throw new Failure("output_limit","Encoded screenshot exceeds companion response limit");
            long completed=SystemClock.uptimeMillis();
            return new JSONObject().put("mime_type",mime).put("format",format).put("width",width).put("height",height).put("native_width",nativeWidth).put("native_height",nativeHeight)
                .put("bytes",bytes.size()).put("image_base64",Base64.encodeToString(bytes.toByteArray(),Base64.NO_WRAP))
                .put("capture_ms",callbackAt-started).put("processing_ms",completed-callbackAt).put("api_timestamp_uptime_ms",result.getTimestamp()).put("api_timestamp_age_ms",Math.max(0,completed-result.getTimestamp()));
        } catch(SecurityException denied) {
            throw new Failure("capture_permission","Android denied screenshot access. Update the phone bridge and check its accessibility status");
        } finally {
            // The Android API has no cancellation handle. Close buffers even when
            // its callback arrives after timeout, client EOF, or session Stop.
            synchronized(pending) {
                if(result==null && pending.isDone() && !pending.isCompletedExceptionally() && !pending.isCancelled()) {
                    AccessibilityService.ScreenshotResult abandoned=pending.getNow(null);
                    if(abandoned!=null)abandoned.getHardwareBuffer().close();
                }
                pending.cancel(false);
            }
            if(resized!=null && resized!=software)resized.recycle();
            if(software!=null)software.recycle();
            if(hardware!=null)hardware.recycle();
            if(result!=null)result.getHardwareBuffer().close();
            lease.ownerFinished();
        }
    }
    }
}
