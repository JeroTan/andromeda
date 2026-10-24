package io.andromeda.companion;

import android.app.Activity;
import android.os.Bundle;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.view.MotionEvent;
import android.view.View;
import android.widget.*;

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
        Switch protectedWindow=new Switch(this);protectedWindow.setText("Protect test window from screenshots");protectedWindow.setMinHeight((int)(48*getResources().getDisplayMetrics().density));
        protectedWindow.setOnCheckedChangeListener((button,checked)->{if(checked)getWindow().addFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE);else getWindow().clearFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE);});layout.addView(protectedWindow);
        layout.addView(pad,new LinearLayout.LayoutParams(-1,0,1));setContentView(layout);
    }
    private final class Pad extends View {
        final Paint paint=new Paint(Paint.ANTI_ALIAS_FLAG);int maxPointers=0,events=0,completed=0;long firstTime=0,lastDuration=0;
        Pad(){super(ProbeActivity.this);setBackgroundColor(Color.rgb(15,23,42));setFocusable(true);setClickable(true);describe();}
        void reset(){maxPointers=events=completed=0;lastDuration=0;describe();invalidate();}
        void describe(){setContentDescription("Gesture pad. max_pointers="+maxPointers+", events="+events+", completed="+completed+", duration_ms="+lastDuration);}
        @Override public boolean onTouchEvent(MotionEvent event){
            events++;maxPointers=Math.max(maxPointers,event.getPointerCount());
            if(event.getActionMasked()==MotionEvent.ACTION_DOWN)firstTime=event.getEventTime();
            if(event.getActionMasked()==MotionEvent.ACTION_UP){completed++;lastDuration=event.getEventTime()-firstTime;performClick();}
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
