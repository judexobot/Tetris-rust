package com.jude.tetrisnesrust

import android.app.Activity
import android.os.Bundle
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.RectF
import android.view.MotionEvent
import android.view.KeyEvent
import android.view.View
import android.view.WindowManager
import kotlin.math.min

class MainActivity : Activity() {
    companion object {
        init { System.loadLibrary("tetris_jni") }
        private const val LEFT = 1
        private const val RIGHT = 2
        private const val DOWN = 4
        private const val A = 8
        private const val B = 16
        private const val START = 32
        private const val SELECT = 64
        private const val UP = 128
    }

    external fun nativeCreate(): Long
    external fun nativeDestroy(handle: Long)
    external fun nativeTick(handle: Long, buttons: Int)
    external fun nativeRender(handle: Long, pixels: IntArray)
    external fun nativeScore(handle: Long): Int
    external fun nativeScene(handle: Long): Int
    external fun nativeGameTypeB(handle: Long): Boolean
    external fun nativeSetBest(handle: Long, a: Int, b: Int)
    private val heldKeys = mutableSetOf<Int>()
    private var lastSavedScore = -1
    private var lastSavedScene = -1

    private var handle = 0L
    private lateinit var gameView: GameView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        @Suppress("DEPRECATION")
        window.decorView.systemUiVisibility = (View.SYSTEM_UI_FLAG_FULLSCREEN
            or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
            or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
            or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
            or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
            or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION)
        window.setFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON,
            WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        handle = nativeCreate()
        val prefs = getSharedPreferences("tetris_scores_v1", MODE_PRIVATE)
        nativeSetBest(handle, prefs.getInt("best_a",0), prefs.getInt("best_b",0))
        gameView = GameView()
        setContentView(gameView)
    }

    override fun onPause() {
        saveHighScore()
        super.onPause()
        gameView.running = false
        heldKeys.clear()
    }
    override fun onResume() {
        super.onResume()
        if (::gameView.isInitialized) { gameView.running = true; gameView.invalidate() }
    }
    override fun onDestroy() {
        saveHighScore()
        gameView.running = false
        if (handle != 0L) { nativeDestroy(handle); handle = 0L }
        super.onDestroy()
    }

    private fun saveHighScore() {
        if (handle == 0L) return
        val score = nativeScore(handle)
        val scene = nativeScene(handle)
        if (score > lastSavedScore || scene != lastSavedScene) {
            val key = if (nativeGameTypeB(handle)) "best_b" else "best_a"
            val prefs = getSharedPreferences("tetris_scores_v1", MODE_PRIVATE)
            if (score > prefs.getInt(key, 0)) prefs.edit().putInt(key, score).apply()
            lastSavedScore=score
            lastSavedScene=scene
        }
    }

    private fun keyToMask(code: Int): Int = when(code) {
        KeyEvent.KEYCODE_DPAD_LEFT, KeyEvent.KEYCODE_A -> LEFT
        KeyEvent.KEYCODE_DPAD_RIGHT, KeyEvent.KEYCODE_D -> RIGHT
        KeyEvent.KEYCODE_DPAD_UP, KeyEvent.KEYCODE_W -> UP
        KeyEvent.KEYCODE_DPAD_DOWN, KeyEvent.KEYCODE_S -> DOWN
        KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_J, KeyEvent.KEYCODE_SPACE -> A
        KeyEvent.KEYCODE_BUTTON_B, KeyEvent.KEYCODE_K -> B
        KeyEvent.KEYCODE_BUTTON_START, KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> START
        KeyEvent.KEYCODE_BUTTON_SELECT, KeyEvent.KEYCODE_SHIFT_LEFT -> SELECT
        else -> 0
    }
    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        if (keyToMask(keyCode) != 0) {
            heldKeys.add(keyCode)
            return true
        }
        return super.onKeyDown(keyCode, event)
    }
    override fun onKeyUp(keyCode: Int, event: KeyEvent): Boolean {
        if (keyToMask(keyCode) != 0) {
            heldKeys.remove(keyCode)
            return true
        }
        return super.onKeyUp(keyCode, event)
    }

    private inner class GameView : View(this@MainActivity) {
        private val bitmap = Bitmap.createBitmap(256, 240, Bitmap.Config.ARGB_8888)
        private val framebuffer = IntArray(256*240)
        private val screenPaint = Paint().apply { isFilterBitmap = false }
        private val uiPaint = Paint(Paint.ANTI_ALIAS_FLAG)
        private val sourceRect = Rect(0, 0, 256, 240)
        private val buttons = ArrayList<TouchButton>()
        private val pointerMasks = mutableMapOf<Int, Int>()
        var running = true
        private var lastTickNanos = 0L
        private var accumulatorNanos = 0L

        init { isClickable = true; isFocusable = true }

        private inner class TouchButton(val mask: Int, val label: String, val rect: RectF)

        private fun controls() {
            buttons.clear()
            val w=width.toFloat()
            val h=height.toFloat()
            val landscape=w>h
            fun add(bit:Int, label:String, cx:Float, cy:Float, bw:Float, bh:Float) {
                buttons.add(TouchButton(bit, label,
                    RectF(w*(cx-bw/2),h*(cy-bh/2),w*(cx+bw/2),h*(cy+bh/2))))
            }
            if (landscape) {
                add(LEFT,"◀",.09f,.69f,.12f,.16f)
                add(RIGHT,"▶",.29f,.69f,.12f,.16f)
                add(UP,"▲",.19f,.48f,.12f,.16f)
                add(DOWN,"▼",.19f,.88f,.12f,.16f)
                add(B,"B",.75f,.72f,.11f,.19f)
                add(A,"A",.90f,.58f,.11f,.19f)
                add(SELECT,"SELECT",.60f,.88f,.13f,.10f)
                add(START,"START",.90f,.89f,.14f,.10f)
            } else {
                add(LEFT,"◀",.10f,.79f,.17f,.075f)
                add(RIGHT,"▶",.38f,.79f,.17f,.075f)
                add(UP,"▲",.24f,.70f,.17f,.075f)
                add(DOWN,"▼",.24f,.88f,.17f,.075f)
                add(B,"B",.65f,.84f,.19f,.090f)
                add(A,"A",.87f,.76f,.19f,.090f)
                add(SELECT,"SELECT",.45f,.96f,.19f,.055f)
                add(START,"START",.74f,.96f,.19f,.055f)
            }
        }

        override fun onSizeChanged(w:Int, h:Int, oldw:Int, oldh:Int) {
            super.onSizeChanged(w,h,oldw,oldh)
            controls()
        }

        override fun onDraw(canvas: Canvas) {
            super.onDraw(canvas)
            canvas.drawColor(Color.rgb(12,14,26))
            if (handle == 0L) return
            if (running) {
                val now = System.nanoTime()
                if (lastTickNanos == 0L) lastTickNanos = now
                accumulatorNanos += (now-lastTickNanos).coerceIn(0L,100_000_000L)
                lastTickNanos = now
                var ticks=0
                while (accumulatorNanos >= 16_666_667L && ticks<4) {
                    val touch = pointerMasks.values.fold(0) { a,b -> a or b }
                    val gamepad = heldKeys.fold(0) { mask, key -> mask or keyToMask(key) }
                    nativeTick(handle, touch or gamepad)
                    accumulatorNanos -= 16_666_667L
                    ticks++
                }
                if (ticks==4) accumulatorNanos=0
                saveHighScore()
                nativeRender(handle,framebuffer)
                bitmap.setPixels(framebuffer,0,256,0,0,256,240)
            }
            val landscape=width>height
            val maxW=width.toFloat()*(if(landscape) .66f else .96f)
            val maxH=height.toFloat()*(if(landscape) .94f else .68f)
            val scale=min(maxW/256f,maxH/240f)
            val dw=256f*scale; val dh=240f*scale
            val dx=(width-dw)/2f
            val dy=if(landscape)(height-dh)/2f else 10f
            uiPaint.color=Color.rgb(48,51,78)
            uiPaint.style=Paint.Style.STROKE
            uiPaint.strokeWidth=2f
            canvas.drawRect(RectF(dx-1,dy-1,dx+dw+1,dy+dh+1),uiPaint)
            uiPaint.style=Paint.Style.FILL
            canvas.drawBitmap(bitmap,sourceRect,RectF(dx,dy,dx+dw,dy+dh),screenPaint)
            for(button in buttons) {
                val active=pointerMasks.values.any {it and button.mask != 0}
                uiPaint.color=if(active) Color.argb(160,113,159,230) else Color.argb(75,116,130,165)
                canvas.drawRoundRect(button.rect,18f,18f,uiPaint)
                uiPaint.color=Color.argb(220,236,240,254)
                uiPaint.textSize=if (button.label.length>2) min(24f,button.rect.height()*.29f)
                                 else min(38f,button.rect.height()*.45f)
                uiPaint.textAlign=Paint.Align.CENTER
                val baseline=button.rect.centerY()-(uiPaint.ascent()+uiPaint.descent())/2
                canvas.drawText(button.label,button.rect.centerX(),baseline,uiPaint)
            }
            if(running) postInvalidateDelayed(8L)
        }

        override fun onTouchEvent(event:MotionEvent):Boolean {
            when(event.actionMasked) {
                MotionEvent.ACTION_DOWN,MotionEvent.ACTION_POINTER_DOWN,
                MotionEvent.ACTION_MOVE,MotionEvent.ACTION_UP,
                MotionEvent.ACTION_POINTER_UP -> {
                    for (n in 0 until event.pointerCount) {
                        val id=event.getPointerId(n)
                        if ((event.actionMasked==MotionEvent.ACTION_POINTER_UP ||
                             event.actionMasked==MotionEvent.ACTION_UP) && n==event.actionIndex) {
                            pointerMasks.remove(id)
                        } else {
                            val px=event.getX(n);val py=event.getY(n)
                            var mask=0
                            for(btn in buttons) {
                                if(btn.rect.contains(px,py))mask=mask or btn.mask
                            }
                            pointerMasks[id]=mask
                        }
                    }
                    invalidate()
                    return true
                }
                MotionEvent.ACTION_CANCEL -> {pointerMasks.clear();invalidate();return true}
            }
            return true
        }
    }
}
