package io.andromeda.companion;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.GestureDescription;
import android.content.SharedPreferences;
import android.graphics.Path;
import android.graphics.Rect;
import android.net.LocalServerSocket;
import android.net.LocalSocket;
import android.net.LocalSocketAddress;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.util.DisplayMetrics;
import android.view.WindowManager;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayDeque;
import java.util.Set;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;

/** ADB-local, paired, explicitly armed accessibility endpoint. No network listener. */
public final class ControlService extends AccessibilityService {
    public static volatile ControlService instance;
    private static final Object ENDPOINT_LOCK = new Object();
    private volatile long pairingUntil=0, armedUntil=0;
    private volatile long lastAuthenticatedAt=0;
    private final AtomicLong generation=new AtomicLong();
    private final AtomicBoolean gestureActive=new AtomicBoolean();
    private final Handler main=new Handler(Looper.getMainLooper());
    private final Set<LocalSocket> clients=ConcurrentHashMap.newKeySet();
    private final ThreadPoolExecutor workers=new ThreadPoolExecutor(4,4,30,TimeUnit.SECONDS,new ArrayBlockingQueue<>(4));
    private volatile LocalServerSocket listener;
    private volatile boolean destroyed;
    private SharedPreferences preferences;

    @Override protected void onServiceConnected(){
        preferences=getSharedPreferences("pairing",MODE_PRIVATE);
        destroyed=false;
        bindEndpoint(0);
    }
    private void bindEndpoint(int attempt){
        LocalServerSocket owned;
        synchronized(ENDPOINT_LOCK){
            if(destroyed)return;
            // Android can call onServiceConnected again on this instance. Reuse the
            // existing listener instead of racing it for the fixed name.
            if(listener!=null){instance=this;return;}
            try{owned=new LocalServerSocket("andromeda-v1");}
            catch(Exception unavailable){
                // The old accept thread may not have released the abstract name yet.
                // Retry without clearing a healthy endpoint owned by another instance.
                if(attempt<20)main.postDelayed(()->bindEndpoint(attempt+1),250);
                return;
            }
            instance=this;
            listener=owned;
        }
        new Thread(()->listen(owned),"andromeda-adb").start();
    }
    @Override public void onAccessibilityEvent(AccessibilityEvent event){}
    @Override public void onInterrupt(){stopSession();}
    @Override public void onDestroy(){
        LocalServerSocket owned=null;
        synchronized(ENDPOINT_LOCK){
            destroyed=true;
            if(instance==this){
                instance=null;
                owned=listener;
                listener=null;
            }
        }
        if(owned!=null){
            // LocalServerSocket.close() does not reliably interrupt a blocked accept().
            // Queue one client so the accept thread wakes, observes the cleared owner,
            // and closes the listener before Android binds the replacement service.
            try(LocalSocket wake=new LocalSocket()){wake.connect(new LocalSocketAddress("andromeda-v1"));}catch(Exception ignored){}
            try{owned.close();}catch(Exception ignored){}
        }
        stopSession();pairingUntil=0;
        for(LocalSocket client:clients)close(client);
        workers.shutdownNow();super.onDestroy();
    }
    public void openPairing(){stopSession();pairingUntil=SystemClock.elapsedRealtime()+60000;}
    public boolean startSession(){
        if(preferences.getString("token","").isEmpty())return false;
        pairingUntil=0;
        generation.incrementAndGet();
        int minutes=preferences.getInt("session_timer_minutes",0);
        armedUntil=minutes==0?Long.MAX_VALUE:SystemClock.elapsedRealtime()+minutes*60_000L;
        return true;
    }
    public void stopSession(){armedUntil=0;pairingUntil=0;generation.incrementAndGet();}
    public boolean hasPairing(){return !preferences.getString("token","").isEmpty();}
    public int pairingSeconds(){return (int)Math.max(0,(pairingUntil-SystemClock.elapsedRealtime()+999)/1000);}
    public boolean ready(){return instance==this && listener!=null;}
    public boolean sessionActive(){return armed();}
    public int sessionSeconds(){return armedUntil==Long.MAX_VALUE?-1:(int)Math.max(0,(armedUntil-SystemClock.elapsedRealtime()+999)/1000);}
    public boolean sessionExpired(){return armedUntil>0 && armedUntil<=SystemClock.elapsedRealtime();}
    public int lastContactSeconds(){return lastAuthenticatedAt==0?-1:(int)Math.min(Integer.MAX_VALUE,(SystemClock.elapsedRealtime()-lastAuthenticatedAt)/1000);}
    public void forgetComputer(){stopSession();if(!preferences.edit().remove("token").commit())throw new IllegalStateException("Could not remove the pairing. Try again.");for(LocalSocket client:clients)close(client);}
    private boolean armed(){return armedUntil>SystemClock.elapsedRealtime();}
    public String statusText(){
        if(pairingUntil>SystemClock.elapsedRealtime())return "Pairing is open for 60 seconds";
        if(armed())return armedUntil==Long.MAX_VALUE?"Session active until you stop it.":"Session active. Your selected timer will stop it automatically.";
        return preferences.getString("token","").isEmpty()?"Accessibility enabled. Pair a computer to continue.":"Computer paired. Session stopped.";
    }

    private void listen(LocalServerSocket owned){
        try{
            while(instance==this && listener==owned){
                LocalSocket client=owned.accept();
                if(instance!=this || listener!=owned){close(client);break;}
                int uid=client.getPeerCredentials().getUid();
                // adbd uses shell on production devices and root on test emulators.
                if(uid!=2000 && uid!=0){close(client);continue;}
                clients.add(client);
                try{workers.execute(()->serve(client));}catch(RejectedExecutionException full){clients.remove(client);close(client);}
            }
        }catch(Exception stopped){
            // No request content or token is written to Android logs.
            if(instance==this)stopSession();
        }finally{
            synchronized(ENDPOINT_LOCK){
                try{owned.close();}catch(Exception ignored){}
                if(listener==owned)listener=null;
            }
        }
    }
    private void serve(LocalSocket socket) {
        AtomicBoolean connected = new AtomicBoolean(true);
        try {
            socket.setSoTimeout(15000);
            DataInputStream input = new DataInputStream(socket.getInputStream());
            DataOutputStream output = new DataOutputStream(socket.getOutputStream());
            int length = input.readInt();
            if (length < 1 || length > 1024 * 1024) return;
            byte[] bytes = new byte[length]; input.readFully(bytes);
            // One request per connection. EOF cancels queued work without stopping other clients.
            Thread disconnect = new Thread(() -> {
                try { input.read(); } catch (Exception closed) { }
                connected.set(false);
            }, "andromeda-disconnect");
            disconnect.setDaemon(true); disconnect.start();
            JSONObject response;
            try { response = handle(new JSONObject(new String(bytes, StandardCharsets.UTF_8)), connected); }
            catch (Problem error) { response = failure(error.code, error.getMessage()); }
            catch (TimeoutException error) { response = failure("timeout", "Companion request exceeded its deadline; observe before retrying input"); }
            catch (Exception error) { response = failure("invalid_request", "Invalid or unsupported companion request"); }
            byte[] encoded = response.toString().getBytes(StandardCharsets.UTF_8);
            if (encoded.length > 4 * 1024 * 1024) encoded = failure("output_limit", "Companion result exceeded 4 MiB").toString().getBytes(StandardCharsets.UTF_8);
            if (connected.get()) { output.writeInt(encoded.length); output.write(encoded); output.flush(); }
        } catch (Exception closed) { }
        finally { connected.set(false); clients.remove(socket); close(socket); }
    }

    private JSONObject handle(JSONObject request, AtomicBoolean connected)throws Exception{
        if(integer(request,"version",1,1)!=1)throw new Problem("protocol_version","Expected companion protocol version 1");
        String method=string(request,"method",32);
        String token=string(request,"token",64);
        if(!token.matches("[0-9a-f]{64}"))throw new Problem("unauthorized","Invalid pairing token");
        if(method.equals("pair")){
            synchronized(this){
                if(pairingUntil<=SystemClock.elapsedRealtime())throw new Problem("pairing_closed","Open pairing on the phone first");
                pairingUntil=0;stopSession();
                if(!preferences.edit().putString("token",token).commit())throw new Problem("pairing_failed","Cannot save pairing on this device");
            }
            return success(new JSONObject().put("paired",true));
        }
        byte[] expected=preferences.getString("token","").getBytes(StandardCharsets.UTF_8);
        if(expected.length!=64 || !MessageDigest.isEqual(expected,token.getBytes(StandardCharsets.UTF_8)))throw new Problem("unauthorized","Pair this computer again from the phone");
        lastAuthenticatedAt=SystemClock.elapsedRealtime();
        if(method.equals("status"))return success(new JSONObject().put("version",1).put("session_active",armed()).put("session_seconds",sessionSeconds()).put("gesture_active",gestureActive.get()));
        if(method.equals("stop")){stopSession();return success(new JSONObject().put("stopped",true).put("note","Already dispatched Android input may finish"));}
        requireSession();long requestGeneration=generation.get();
        return switch(method){
            case "ui_tree" -> success(uiTree(requestGeneration, connected));
            case "type_text" -> success(typeTextVerified(request,requestGeneration,connected));
            case "gesture" -> success(gesture(request,requestGeneration,connected));
            default -> throw new Problem("unknown_method","Unknown companion method");
        };
    }

    private void requireSession()throws Problem{if(!armed())throw new Problem("session_inactive","Start a session in the Andromeda app on the phone");}
    private void checkRequest(long expectedGeneration, AtomicBoolean connected)throws Problem{requireSession();if(!connected.get() || generation.get()!=expectedGeneration)throw new Problem("cancelled","The request or session ended before completion");}
    private <T>T onMain(Callable<T> action,long timeout,long requestGeneration,AtomicBoolean connected)throws Exception{
        FutureTask<T> task=new FutureTask<>(()->{checkRequest(requestGeneration,connected);return action.call();});
        main.post(task);
        try{return task.get(timeout,TimeUnit.MILLISECONDS);}
        catch(ExecutionException failure){if(failure.getCause() instanceof Exception error)throw error;throw failure;}
        finally{if(!task.isDone()){task.cancel(false);main.removeCallbacks(task);}}
    }
    private JSONObject screen()throws JSONException{
        WindowManager manager=(WindowManager)getSystemService(WINDOW_SERVICE);
        DisplayMetrics metrics=new DisplayMetrics();manager.getDefaultDisplay().getRealMetrics(metrics);
        return new JSONObject().put("width",metrics.widthPixels).put("height",metrics.heightPixels).put("rotation",manager.getDefaultDisplay().getRotation());
    }
    private void checkScreen(JSONObject expected,JSONObject actual)throws Exception{
        for(String key:new String[]{"width","height","rotation"}){
            if(integer(expected,key,0,16384)!=actual.getInt(key))throw new Problem("stale_screen","Screen geometry changed; observe again");
        }
    }
    private record TextChange(String expected, int cursor, int characters, String mode, boolean masked) {}
    private String focusedText() throws Problem {
        AccessibilityNodeInfo root=getRootInActiveWindow();
        if(root==null)throw new Problem("ui_unavailable","No active accessibility window");
        AccessibilityNodeInfo focused=root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        if(focused==null || !focused.isEditable())throw new Problem("no_editable_focus","Focus an editable field first");
        return focused.isShowingHintText()?"":value(focused.getText());
    }
    private JSONObject typeTextVerified(JSONObject request,long expectedGeneration,AtomicBoolean connected)throws Exception{
        TextChange change=onMain(()->typeText(request),3000,expectedGeneration,connected);
        if(change.masked()){
            return new JSONObject().put("characters",change.characters()).put("mode",change.mode())
                .put("verified",false).put("note","Password text is masked by Android; inspect the app outcome before continuing");
        }
        long deadline=SystemClock.elapsedRealtime()+750;
        while(SystemClock.elapsedRealtime()<deadline){
            checkRequest(expectedGeneration,connected);
            String current=onMain(this::focusedText,1000,expectedGeneration,connected);
            if(current.equals(change.expected())){
                boolean positioned=onMain(()->{
                    AccessibilityNodeInfo root=getRootInActiveWindow();
                    AccessibilityNodeInfo focused=root==null?null:root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
                    if(focused==null)return false;
                    int at=change.cursor();
                    if(focused.getTextSelectionStart()==at && focused.getTextSelectionEnd()==at)return true;
                    Bundle selection=new Bundle();
                    selection.putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_START_INT,at);
                    selection.putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_END_INT,at);
                    focused.performAction(AccessibilityNodeInfo.ACTION_SET_SELECTION,selection);
                    return focused.getTextSelectionStart()==at && focused.getTextSelectionEnd()==at;
                },1000,expectedGeneration,connected);
                return new JSONObject().put("characters",change.characters()).put("mode",change.mode())
                    .put("cursor_positioned",positioned).put("verified",true);
            }
            SystemClock.sleep(25);
        }
        throw new Problem("text_unverified","The focused field did not show the requested text; inspect it before retrying");
    }
    private TextChange typeText(JSONObject request)throws Exception{
        requireSession();
        String text=string(request,"text",16384);
        AccessibilityNodeInfo root=getRootInActiveWindow();
        if(root==null)throw new Problem("ui_unavailable","No active accessibility window");
        AccessibilityNodeInfo focused=root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        if(focused==null || !focused.isEditable())throw new Problem("no_editable_focus","Focus an editable field first");
        String mode=request.optString("mode","insert");
        if(!mode.equals("insert") && !mode.equals("replace"))throw new Problem("invalid_argument","Text mode must be insert or replace");
        int inserted=text.codePointCount(0,text.length());
        int cursor=text.length();
        if(mode.equals("insert")){
            if(focused.isPassword())throw new Problem("sensitive_field","Use explicit replacement mode for a password field");
            CharSequence current=focused.isShowingHintText()?"":focused.getText();
            if(current!=null && current.length()>16384)throw new Problem("output_limit","Focused text exceeds the insertion limit");
            String existing=value(current);
            int start=focused.getTextSelectionStart(),end=focused.getTextSelectionEnd();
            if(start<0 || end<0){start=end=existing.length();}
            if(start>existing.length() || end>existing.length())throw new Problem("stale_focus","Text selection changed; inspect the field again");
            cursor=Math.min(start,end)+text.length();
            text=existing.substring(0,Math.min(start,end))+text+existing.substring(Math.max(start,end));
            if(text.length()>16384)throw new Problem("output_limit","Resulting text exceeds 16384 characters");
        }
        Bundle arguments=new Bundle();arguments.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,text);
        if(!focused.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT,arguments))throw new Problem("text_rejected","The focused app rejected text replacement");
        return new TextChange(text,cursor,inserted,mode,focused.isPassword());
    }
    private JSONObject gesture(JSONObject request,long expectedGeneration,AtomicBoolean connected)throws Exception{
        if(!gestureActive.compareAndSet(false,true))throw new Problem("device_busy","A companion gesture is already active");
        CompletableFuture<JSONObject> completion=new CompletableFuture<>();
        try{
            onMain(()->{
                JSONObject current=screen();checkScreen(request.getJSONObject("screen"),current);
                JSONArray strokes=request.getJSONArray("strokes");
                if(strokes.length()<1 || strokes.length()>GestureDescription.getMaxStrokeCount())throw new Problem("invalid_argument","Gesture stroke count outside Android limits");
                GestureDescription.Builder gesture=new GestureDescription.Builder();
                for(int i=0;i<strokes.length();i++){
                    JSONObject stroke=strokes.getJSONObject(i);
                    long start=integer(stroke,"start_ms",0,10000),duration=integer(stroke,"duration_ms",1,10000);
                    if(start+duration>10000)throw new Problem("invalid_argument","Gesture must finish within 10000 ms");
                    JSONArray points=stroke.getJSONArray("points");
                    if(points.length()<1 || points.length()>128)throw new Problem("invalid_argument","Each stroke needs 1 to 128 points");
                    Path path=new Path();
                    for(int j=0;j<points.length();j++){
                        JSONObject point=points.getJSONObject(j);
                        int x=integer(point,"x",0,current.getInt("width")-1),y=integer(point,"y",0,current.getInt("height")-1);
                        if(j==0)path.moveTo(x,y);else path.lineTo(x,y);
                    }
                    gesture.addStroke(new GestureDescription.StrokeDescription(path,start,duration));
                }
                boolean accepted=dispatchGesture(gesture.build(),new GestureResultCallback(){
                    @Override public void onCompleted(GestureDescription result){
                        gestureActive.set(false);
                        if(generation.get()!=expectedGeneration){completion.completeExceptionally(new Problem("cancelled","Session stopped; input may already have occurred"));return;}
                        try{completion.complete(new JSONObject().put("completed",true).put("strokes",strokes.length()));}catch(JSONException error){completion.completeExceptionally(error);}
                    }
                    @Override public void onCancelled(GestureDescription result){gestureActive.set(false);completion.completeExceptionally(new Problem("gesture_cancelled","Android cancelled the gesture; observe before retrying"));}
                },main);
                if(!accepted)throw new Problem("gesture_rejected","Android rejected gesture dispatch; check accessibility permission");
                return true;
            },3000,expectedGeneration,connected);
        }catch(Exception error){gestureActive.set(false);throw error;}
        long deadline = SystemClock.elapsedRealtime() + 12000;
        while (SystemClock.elapsedRealtime() < deadline) {
            checkRequest(expectedGeneration, connected);
            try { return completion.get(25, TimeUnit.MILLISECONDS); }
            catch (TimeoutException waiting) { }
            catch (ExecutionException failure) { if (failure.getCause() instanceof Exception error) throw error; throw failure; }
        }
        throw new TimeoutException("Gesture completion deadline exceeded");
    }

    private record PendingNode(AccessibilityNodeInfo node,int parent){}
    private JSONObject uiTree(long expectedGeneration,AtomicBoolean connected)throws Exception{
        long deadline=SystemClock.elapsedRealtime()+3000;
        requireSession();JSONObject geometry=screen();
        AccessibilityNodeInfo root=getRootInActiveWindow();
        if(root==null)throw new Problem("ui_unavailable","No active accessibility window");
        JSONArray nodes=new JSONArray();ArrayDeque<PendingNode> pending=new ArrayDeque<>();pending.add(new PendingNode(root,-1));int textBudget=0;
        while(!pending.isEmpty()){
            checkRequest(expectedGeneration,connected);
            if(SystemClock.elapsedRealtime()>deadline)throw new TimeoutException("UI inspection deadline exceeded");
            if(nodes.length()+pending.size()>10000)throw new Problem("output_limit","UI tree exceeds 10000 nodes");
            PendingNode entry=pending.removeFirst();AccessibilityNodeInfo node=entry.node();int id=nodes.length();
            Rect bounds=new Rect();node.getBoundsInScreen(bounds);boolean password=node.isPassword();
            String text=password?"":bounded(node.getText()),description=password?"":bounded(node.getContentDescription());
            textBudget+=text.length()+description.length();
            if(textBudget>1024*1024)throw new Problem("output_limit","UI text exceeds 1 MiB");
            JSONObject item=new JSONObject().put("id",id).put("parent",entry.parent()<0?JSONObject.NULL:entry.parent())
                .put("text",text).put("description",description)
                .put("resource_id",bounded(node.getViewIdResourceName())).put("class",bounded(node.getClassName())).put("package",bounded(node.getPackageName()))
                .put("bounds",new JSONArray(new int[]{bounds.left,bounds.top,bounds.right,bounds.bottom}))
                .put("clickable",node.isClickable()).put("enabled",node.isEnabled()).put("password",password);
            nodes.put(item);
            for(int i=0;i<node.getChildCount();i++){if(nodes.length()+pending.size()>=10000)throw new Problem("output_limit","UI tree exceeds 10000 nodes");AccessibilityNodeInfo child=node.getChild(i);if(child!=null)pending.add(new PendingNode(child,id));}
            node.recycle();
        }
        checkScreen(geometry,screen());
        return new JSONObject().put("screen",geometry).put("nodes",nodes);
    }
    private static String value(CharSequence text){return text==null?"":text.toString();}
    private static String bounded(CharSequence text)throws Problem{if(text!=null && text.length()>4096)throw new Problem("output_limit","A UI node field exceeds 4096 characters");return value(text);}
    private static int integer(JSONObject object,String name,int min,int max)throws Exception{
        Object raw=object.get(name);
        if(!(raw instanceof Number number) || number.doubleValue()!=number.longValue() || number.longValue()<min || number.longValue()>max)throw new Problem("invalid_argument","Invalid integer field: "+name);
        return number.intValue();
    }
    private static String string(JSONObject object,String name,int max)throws Exception{
        Object raw=object.get(name);if(!(raw instanceof String text) || text.length()>max)throw new Problem("invalid_argument","Invalid string field: "+name);return text;
    }
    private static JSONObject success(JSONObject data)throws JSONException{return new JSONObject().put("success",true).put("data",data);}
    private static JSONObject failure(String code,String message){try{return new JSONObject().put("success",false).put("error",new JSONObject().put("code",code).put("message",message));}catch(JSONException impossible){throw new IllegalStateException(impossible);}}
    private static void close(LocalSocket socket){try{socket.close();}catch(Exception ignored){}}
    private static final class Problem extends Exception{final String code;Problem(String code,String message){super(message);this.code=code;}}
}
