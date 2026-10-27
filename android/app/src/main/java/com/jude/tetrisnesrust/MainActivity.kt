package com.jude.tetrisnesrust

import android.app.Activity
import android.app.AlertDialog
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.RectF
import android.os.Bundle
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.SeekBar
import android.widget.TextView
import kotlin.math.min

/** Android owns lifecycle, input and preferences. Game rules and rendering live in Rust. */
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
    external fun nativePause(handle: Long)
    external fun nativeRender(handle: Long, pixels: IntArray)
    external fun nativeBest(handle: Long, typeB: Boolean): Int
    external fun nativeSetBest(handle: Long, a: Int, b: Int)
    external fun nativeRecord(handle: Long, typeB: Boolean, row: Int): Long
    external fun nativeSetRecord(handle: Long, typeB: Boolean, row: Int, value: Long)

    private var handle = 0L
    private lateinit var gameView: GameView
    private val heldKeys = mutableSetOf<Int>()
    private var analogMask = 0
    private var resumed = false
    private val lastRecords = LongArray(6)
    private val scorePrefs by lazy { getSharedPreferences("tetris_scores_v1", MODE_PRIVATE) }
    private val controlPrefs by lazy { getSharedPreferences("tetris_controls_v1", MODE_PRIVATE) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        @Suppress("DEPRECATION")
        window.decorView.systemUiVisibility = (View.SYSTEM_UI_FLAG_FULLSCREEN
            or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
            or View.SYSTEM_UI_FLAG_LAYOUT_STABLE or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
            or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        handle = nativeCreate()
        check(handle != 0L) { "Rust game engine could not be created" }
        nativeSetBest(handle, scorePrefs.getInt("best_a", 0), scorePrefs.getInt("best_b", 0))
        for (index in 0 until 6) {
            val key = "record_$index"
            if (scorePrefs.contains(key)) nativeSetRecord(handle, index >= 3, index % 3, scorePrefs.getLong(key, 0))
            lastRecords[index] = nativeRecord(handle, index >= 3, index % 3)
        }
        gameView = GameView()
        setContentView(gameView)
    }

    override fun onResume() {
        super.onResume()
        resumed = true
        if (::gameView.isInitialized && hasWindowFocus()) gameView.start()
    }

    override fun onPause() {
        resumed = false
        suspendGame()
        saveHighScores()
        super.onPause()
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (!::gameView.isInitialized) return
        if (hasFocus && resumed) gameView.start() else suspendGame()
    }

    private fun suspendGame() {
        if (::gameView.isInitialized) gameView.stop()
        heldKeys.clear()
        analogMask = 0
        if (handle != 0L) nativePause(handle)
    }

    override fun onDestroy() {
        saveHighScores()
        if (::gameView.isInitialized) gameView.stop()
        if (handle != 0L) { nativeDestroy(handle); handle = 0L }
        super.onDestroy()
    }

    private fun saveHighScores() {
        if (handle == 0L) return
        val a = nativeBest(handle, false)
        val b = nativeBest(handle, true)
        var editor: android.content.SharedPreferences.Editor? = null
        if (a != scorePrefs.getInt("best_a", 0) || b != scorePrefs.getInt("best_b", 0)) {
            editor = scorePrefs.edit().putInt("best_a", a).putInt("best_b", b)
        }
        for (index in 0 until 6) {
            val record = nativeRecord(handle, index >= 3, index % 3)
            if (record != lastRecords[index]) {
                if (editor == null) editor = scorePrefs.edit()
                editor.putLong("record_$index", record)
                lastRecords[index] = record
            }
        }
        editor?.apply()
    }

    private fun keyToMask(code: Int): Int = when (code) {
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
        if (keyToMask(keyCode) != 0) { heldKeys.add(keyCode); return true }
        return super.onKeyDown(keyCode, event)
    }

    override fun onKeyUp(keyCode: Int, event: KeyEvent): Boolean {
        if (keyToMask(keyCode) != 0) { heldKeys.remove(keyCode); return true }
        return super.onKeyUp(keyCode, event)
    }

    override fun onGenericMotionEvent(event: MotionEvent): Boolean {
        if (event.action == MotionEvent.ACTION_MOVE &&
            event.isFromSource(InputDevice.SOURCE_JOYSTICK)) {
            val x = event.getAxisValue(MotionEvent.AXIS_X)
            val y = event.getAxisValue(MotionEvent.AXIS_Y)
            val hx = event.getAxisValue(MotionEvent.AXIS_HAT_X)
            val hy = event.getAxisValue(MotionEvent.AXIS_HAT_Y)
            analogMask = (if (x < -.5f || hx < -.5f) LEFT else 0) or
                (if (x > .5f || hx > .5f) RIGHT else 0) or
                (if (y < -.5f || hy < -.5f) UP else 0) or
                (if (y > .5f || hy > .5f) DOWN else 0)
            return true
        }
        return super.onGenericMotionEvent(event)
    }

    private fun showControlSettings() {
        val layout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            val pad = (24 * resources.displayMetrics.density).toInt()
            setPadding(pad, pad / 2, pad, 0)
        }
        fun slider(label: String, key: String, default: Int, max: Int) {
            val text = TextView(this).apply { this.text = label }
            val bar = SeekBar(this).apply {
                this.max = max
                progress = controlPrefs.getInt(key, default).coerceIn(0, max)
                setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
                    override fun onProgressChanged(seekBar: SeekBar, value: Int, fromUser: Boolean) {
                        if (fromUser) {
                            controlPrefs.edit().putInt(key, value).apply()
                            gameView.rebuildControls()
                        }
                    }
                    override fun onStartTrackingTouch(seekBar: SeekBar) = Unit
                    override fun onStopTrackingTouch(seekBar: SeekBar) = Unit
                })
            }
            layout.addView(text)
            layout.addView(bar)
        }
        slider("Tamaño", "size", 50, 100)
        slider("Opacidad", "opacity", 45, 100)
        AlertDialog.Builder(this)
            .setTitle("Controles")
            .setMessage("Mueve cada botón por separado en cada orientación. Pulsa START para reanudar después de cambiar los controles.")
            .setView(layout)
            .setPositiveButton("Mover botones") { _, _ -> gameView.editMode = true; gameView.invalidate() }
            .setNeutralButton("Restablecer") { _, _ -> controlPrefs.edit().clear().apply(); gameView.rebuildControls() }
            .setNegativeButton("Cerrar", null)
            .show()
    }

    private inner class GameView : View(this@MainActivity) {
        private val bitmap = Bitmap.createBitmap(256, 240, Bitmap.Config.ARGB_8888)
        private val framebuffer = IntArray(256 * 240)
        private val screenPaint = Paint().apply { isFilterBitmap = false }
        private val uiPaint = Paint(Paint.ANTI_ALIAS_FLAG)
        private val sourceRect = Rect(0, 0, 256, 240)
        private val buttons = ArrayList<TouchButton>()
        private val pointerMasks = mutableMapOf<Int, Int>()
        private var editingPointer = -1
        private var editingButton: TouchButton? = null
        private var running = false
        private var lastTickNanos = 0L
        private var accumulatorNanos = 0L
        var editMode = false
        private val settingsRect = RectF()
        private val orientationKey: String get() = if (width > height) "land" else "port"

        private inner class TouchButton(val mask: Int, val label: String, val rect: RectF)

        init { isClickable = true; isFocusable = true }

        fun start() {
            if (!running) {
                running = true
                lastTickNanos = 0L
                accumulatorNanos = 0L
                postInvalidateOnAnimation()
            }
        }

        fun stop() {
            running = false
            lastTickNanos = 0L
            accumulatorNanos = 0L
            pointerMasks.clear()
            editingPointer = -1
            editingButton = null
            invalidate()
        }

        fun rebuildControls() {
            buttons.clear()
            if (width == 0 || height == 0) return
            val w = width.toFloat()
            val h = height.toFloat()
            val landscape = w > h
            val scale = .75f + controlPrefs.getInt("size", 50).coerceIn(0, 100) / 200f
            fun add(bit: Int, label: String, cx: Float, cy: Float, bw: Float, bh: Float) {
                val key = "$orientationKey.$bit"
                val x = controlPrefs.getFloat("$key.x", cx).coerceIn(0f, 1f)
                val y = controlPrefs.getFloat("$key.y", cy).coerceIn(0f, 1f)
                val halfW = w * bw * scale / 2
                val halfH = h * bh * scale / 2
                buttons.add(TouchButton(bit, label, RectF(w*x-halfW, h*y-halfH, w*x+halfW, h*y+halfH)))
            }
            if (landscape) {
                add(LEFT, "◀", .09f, .69f, .12f, .16f)
                add(RIGHT, "▶", .29f, .69f, .12f, .16f)
                add(UP, "▲", .19f, .48f, .12f, .16f)
                add(DOWN, "▼", .19f, .88f, .12f, .16f)
                add(B, "B", .75f, .72f, .11f, .19f)
                add(A, "A", .90f, .58f, .11f, .19f)
                add(SELECT, "SELECT", .60f, .88f, .13f, .10f)
                add(START, "START", .90f, .89f, .14f, .10f)
            } else {
                add(LEFT, "◀", .10f, .79f, .17f, .075f)
                add(RIGHT, "▶", .38f, .79f, .17f, .075f)
                add(UP, "▲", .24f, .70f, .17f, .075f)
                add(DOWN, "▼", .24f, .88f, .17f, .075f)
                add(B, "B", .65f, .84f, .19f, .090f)
                add(A, "A", .87f, .76f, .19f, .090f)
                add(SELECT, "SELECT", .45f, .96f, .19f, .055f)
                add(START, "START", .74f, .96f, .19f, .055f)
            }
            val size = 48f * resources.displayMetrics.density
            settingsRect.set(w - size, 0f, w, size)
            invalidate()
        }

        override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
            super.onSizeChanged(w, h, oldw, oldh)
            pointerMasks.clear()
            rebuildControls()
        }

        override fun onDraw(canvas: Canvas) {
            super.onDraw(canvas)
            canvas.drawColor(Color.BLACK)
            if (handle == 0L) return
            if (running) {
                val now = System.nanoTime()
                if (lastTickNanos == 0L) lastTickNanos = now
                accumulatorNanos += (now - lastTickNanos).coerceIn(0L, 250_000_000L)
                lastTickNanos = now
                var ticks = 0
                while (accumulatorNanos >= 16_666_667L && ticks < 8) {
                    val touch = pointerMasks.values.fold(0) { a, b -> a or b }
                    val gamepad = heldKeys.fold(analogMask) { mask, key -> mask or keyToMask(key) }
                    nativeTick(handle, touch or gamepad)
                    accumulatorNanos -= 16_666_667L
                    ticks++
                }
                // Avoid a spiral of catch-up after a long Android UI stall.
                if (ticks == 8) accumulatorNanos = accumulatorNanos.coerceAtMost(16_666_666L)
                saveHighScores()
                nativeRender(handle, framebuffer)
                bitmap.setPixels(framebuffer, 0, 256, 0, 0, 256, 240)
            }
            val landscape = width > height
            val maxW = width.toFloat() * if (landscape) .66f else .96f
            val maxH = height.toFloat() * if (landscape) .94f else .68f
            val scale = min(maxW / 256f, maxH / 240f)
            val dw = 256f * scale
            val dh = 240f * scale
            val dx = (width - dw) / 2f
            val dy = if (landscape) (height - dh) / 2f else 10f
            canvas.drawBitmap(bitmap, sourceRect, RectF(dx, dy, dx + dw, dy + dh), screenPaint)
            val alpha = (controlPrefs.getInt("opacity", 45).coerceIn(0, 100) * 2.15f + 25).toInt()
            for (button in buttons) {
                val active = pointerMasks.values.any { it and button.mask != 0 }
                uiPaint.color = if (active || editMode) Color.argb((alpha + 50).coerceAtMost(255), 113, 159, 230)
                    else Color.argb(alpha, 116, 130, 165)
                canvas.drawRoundRect(button.rect, 18f, 18f, uiPaint)
                uiPaint.color = Color.argb((alpha + 60).coerceAtMost(255), 236, 240, 254)
                uiPaint.textSize = if (button.label.length > 2) min(24f, button.rect.height() * .29f)
                    else min(38f, button.rect.height() * .45f)
                uiPaint.textAlign = Paint.Align.CENTER
                val baseline = button.rect.centerY() - (uiPaint.ascent() + uiPaint.descent()) / 2
                canvas.drawText(button.label, button.rect.centerX(), baseline, uiPaint)
            }
            uiPaint.color = Color.argb(180, 60, 70, 90)
            canvas.drawRoundRect(settingsRect, 12f, 12f, uiPaint)
            uiPaint.color = Color.WHITE
            uiPaint.textSize = settingsRect.height() * if (editMode) .28f else .45f
            uiPaint.textAlign = Paint.Align.CENTER
            canvas.drawText(if (editMode) "OK" else "⚙", settingsRect.centerX(),
                settingsRect.centerY() - (uiPaint.ascent() + uiPaint.descent()) / 2, uiPaint)
            if (running) postInvalidateOnAnimation()
        }

        override fun performClick(): Boolean { super.performClick(); return true }

        override fun onTouchEvent(event: MotionEvent): Boolean {
            if (event.actionMasked == MotionEvent.ACTION_DOWN && settingsRect.contains(event.x, event.y)) {
                performClick()
                if (editMode) { editMode = false; invalidate() } else showControlSettings()
                return true
            }
            if (editMode) {
                when (event.actionMasked) {
                    MotionEvent.ACTION_DOWN -> {
                        editingPointer = event.getPointerId(0)
                        editingButton = buttons.lastOrNull { it.rect.contains(event.x, event.y) }
                    }
                    MotionEvent.ACTION_MOVE -> {
                        val index = event.findPointerIndex(editingPointer)
                        val button = editingButton
                        if (index >= 0 && button != null) {
                            val key = "$orientationKey.${button.mask}"
                            controlPrefs.edit().putFloat("$key.x", (event.getX(index) / width).coerceIn(0f, 1f))
                                .putFloat("$key.y", (event.getY(index) / height).coerceIn(0f, 1f)).apply()
                            val bit = button.mask
                            rebuildControls()
                            editingButton = buttons.firstOrNull { it.mask == bit }
                        }
                    }
                    MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> { editingPointer = -1; editingButton = null }
                }
                return true
            }
            when (event.actionMasked) {
                MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN, MotionEvent.ACTION_MOVE,
                MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                    for (n in 0 until event.pointerCount) {
                        val id = event.getPointerId(n)
                        if ((event.actionMasked == MotionEvent.ACTION_POINTER_UP || event.actionMasked == MotionEvent.ACTION_UP)
                            && n == event.actionIndex) pointerMasks.remove(id)
                        else {
                            var mask = 0
                            for (btn in buttons) if (btn.rect.contains(event.getX(n), event.getY(n))) mask = mask or btn.mask
                            pointerMasks[id] = mask
                        }
                    }
                    invalidate()
                }
                MotionEvent.ACTION_CANCEL -> { pointerMasks.clear(); invalidate() }
            }
            return true
        }
    }
}
