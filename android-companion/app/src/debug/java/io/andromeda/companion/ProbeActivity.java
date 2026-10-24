package io.andromeda.companion;

import android.app.Activity;
import android.app.AlertDialog;
import android.os.Bundle;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.view.MotionEvent;
import android.view.View;
import android.widget.*;
import org.json.JSONArray;
import org.json.JSONObject;
import java.util.LinkedHashMap;
import java.util.Map;

/** Local test surface: accessible event counters verify actual Android input. */
public final class ProbeActivity extends Activity {
    @Override public void onCreate(Bundle state){
        super.onCreate(state);
        LinearLayout layout=new LinearLayout(this);layout.setOrientation(LinearLayout.VERTICAL);
        layout.setPadding(24,24,24,24);
        layout.setOnApplyWindowInsetsListener((view,insets)->{layout.setPadding(24,24+insets.getSystemWindowInsetTop(),24,24+insets.getSystemWindowInsetBottom());return insets;});
        TextView title=new TextView(this);title.setText("Gesture test pad");title.setTextSize(26);layout.addView(title);
        EditText text=new EditText(this);text.setId(R.id.unicode_input);text.setHint("Type or replace Unicode text here");text.setSingleLine(false);text.setMinHeight(64);layout.addView(text,new LinearLayout.LayoutParams(-1,-2));
        Pad pad=new Pad();pad.setId(R.id.gesture_pad);
        Button reset=new Button(this);reset.setText("Reset gesture counters");reset.setAllCaps(false);reset.setOnClickListener(v->pad.reset());layout.addView(reset);
        Button dialog=new Button(this);dialog.setText("Open test dialog");dialog.setAllCaps(false);
        dialog.setOnClickListener(v->new AlertDialog.Builder(this).setTitle("Andromeda test dialog").setMessage("Close this dialog to finish the test task.").setPositiveButton("Close test dialog",(d,w)->pad.dialogClosed()).show());layout.addView(dialog);
        Switch protectedWindow=new Switch(this);protectedWindow.setText("Protect test window from screenshots");protectedWindow.setMinHeight((int)(48*getResources().getDisplayMetrics().density));
        protectedWindow.setOnCheckedChangeListener((button,checked)->{if(checked)getWindow().addFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE);else getWindow().clearFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE);});layout.addView(protectedWindow);
        layout.addView(pad,new LinearLayout.LayoutParams(-1,0,1));setContentView(layout);
    }
    private final class Pad extends View {
        final Paint paint=new Paint(Paint.ANTI_ALIAS_FLAG);int maxPointers=0,events=0,completed=0,dialogsClosed=0;long firstTime=0,lastDuration=0;
        final Map<Integer,Trace> traces=new LinkedHashMap<>();String lastGesture="[]";
        final class Trace {
            final float startX,startY;float endX,endY;final long started;long ended;int samples;
            Trace(float x,float y,long time){startX=endX=x;startY=endY=y;started=ended=time;}
            JSONObject json()throws Exception{return new JSONObject().put("start_x",startX).put("start_y",startY).put("end_x",endX).put("end_y",endY).put("duration_ms",ended-started).put("samples",samples);}
        }
        Pad(){super(ProbeActivity.this);setBackgroundColor(Color.rgb(15,23,42));setFocusable(true);setClickable(true);describe();}
        void reset(){maxPointers=events=completed=dialogsClosed=0;lastDuration=0;traces.clear();lastGesture="[]";describe();invalidate();}
        void dialogClosed(){dialogsClosed++;describe();}
        void describe(){setContentDescription("Gesture pad. max_pointers="+maxPointers+", events="+events+", completed="+completed+", duration_ms="+lastDuration+", dialogs_closed="+dialogsClosed+", gesture_json="+lastGesture);}
        @Override public boolean onTouchEvent(MotionEvent event){
            events++;maxPointers=Math.max(maxPointers,event.getPointerCount());
            if(event.getActionMasked()==MotionEvent.ACTION_DOWN){firstTime=event.getEventTime();traces.clear();}
            // Use the event's raw offset, not the view's current animated location.
            // The latter may move between Android creating and delivering the event.
            float offsetX=event.getRawX()-event.getX(),offsetY=event.getRawY()-event.getY();
            for(int i=0;i<event.getPointerCount();i++){
                int id=event.getPointerId(i);float x=offsetX+event.getX(i),y=offsetY+event.getY(i);long time=event.getEventTime();
                Trace trace=traces.get(id);if(trace==null){trace=new Trace(x,y,time);traces.put(id,trace);}
                trace.endX=x;trace.endY=y;trace.ended=time;trace.samples++;
            }
            if(event.getActionMasked()==MotionEvent.ACTION_UP){
                completed++;lastDuration=event.getEventTime()-firstTime;
                try{JSONArray trace=new JSONArray();for(Trace pointer:traces.values())trace.put(pointer.json());lastGesture=trace.toString();}catch(Exception error){lastGesture="[]";}
                performClick();
            }
            if(event.getActionMasked()==MotionEvent.ACTION_CANCEL){lastGesture="[]";traces.clear();}
            describe();invalidate();return true;
        }
        @Override public boolean performClick(){super.performClick();return true;}
        @Override protected void onDraw(Canvas canvas){
            super.onDraw(canvas);paint.setColor(Color.rgb(248,250,252));paint.setTextSize(20*getResources().getDisplayMetrics().scaledDensity);
            canvas.drawText("Touch and swipe in this area",24,60,paint);
            canvas.drawText("Maximum pointers: "+maxPointers,24,110,paint);
            canvas.drawText("Completed gestures: "+completed,24,160,paint);
            canvas.drawText("Last duration: "+lastDuration+" ms",24,210,paint);
        }
    }
}
