package tech.lolli.toolbox;

import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.res.Configuration;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.OpenableColumns;
import android.util.Log;
import android.view.View;
import android.view.WindowInsets;
import android.widget.Toast;

import com.google.androidgamesdk.GameActivity;

import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.HashMap;
import java.util.Locale;
import java.util.Map;

import tech.lolli.toolbox.widget.HomeWidget;
import tech.lolli.toolbox.widget.WidgetStore;

/**
 * The whole app is native (Rust, drawn with Vulkan); this activity only hosts
 * it and answers the few things the Android framework does in Java. Every
 * public method here is called from Rust over JNI (`app/src/platform/jni.rs`)
 * on the native thread; none holds app logic.
 */
public class MainActivity extends GameActivity {
    private static final String TAG = "RUSerBox";
    /** Above this a picked file is refused rather than read into memory. */
    private static final long MAX_PICK = 512L * 1024 * 1024;
    private static final int SAVE_REQUEST = 0x5a5e;

    static {
        System.loadLibrary("ruserbox");
    }

    private native void nativeFilePicked(int request, String name, byte[] data);

    private final Map<Integer, Boolean> pickRequests = new HashMap<>();
    private byte[] pendingSave;
    private volatile int[] insets = new int[4];
    private volatile boolean keepAlive;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        View content = getWindow().getDecorView();
        content.setOnApplyWindowInsetsListener((v, wi) -> {
            int types = WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout() | WindowInsets.Type.ime();
            android.graphics.Insets i = wi.getInsets(types);
            insets = new int[] { i.left, i.top, i.right, i.bottom };
            return v.onApplyWindowInsets(wi);
        });
    }

    // ---- called from Rust

    public int[] insets() {
        return insets;
    }

    public boolean isDarkMode() {
        int night = getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK;
        return night == Configuration.UI_MODE_NIGHT_YES;
    }

    public String localeTag() {
        return Locale.getDefault().toLanguageTag();
    }

    /** Where the Flutter build kept `store.db` (`app_flutter`): kept, so an update opens the same file. */
    public String storeDir() {
        return getDir("flutter", Context.MODE_PRIVATE).getAbsolutePath();
    }

    public void setClipboard(String text) {
        runOnUiThread(() -> {
            ClipboardManager cm = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
            if (cm != null) cm.setPrimaryClip(ClipData.newPlainText("RUSerBox", text));
        });
    }

    public String getClipboard() {
        ClipboardManager cm = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (cm == null || !cm.hasPrimaryClip()) return null;
        ClipData clip = cm.getPrimaryClip();
        if (clip == null || clip.getItemCount() == 0) return null;
        CharSequence text = clip.getItemAt(0).coerceToText(this);
        return text == null ? null : text.toString();
    }

    public void pickFile(int request) {
        runOnUiThread(() -> {
            Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            i.addCategory(Intent.CATEGORY_OPENABLE);
            i.setType("*/*");
            pickRequests.put(request, true);
            startActivityForResult(i, request);
        });
    }

    public void saveFile(String name, byte[] data) {
        runOnUiThread(() -> {
            pendingSave = data;
            Intent i = new Intent(Intent.ACTION_CREATE_DOCUMENT);
            i.addCategory(Intent.CATEGORY_OPENABLE);
            i.setType("application/octet-stream");
            i.putExtra(Intent.EXTRA_TITLE, name);
            startActivityForResult(i, SAVE_REQUEST);
        });
    }

    /** Whether open sessions should survive the app going to the background. */
    public void keepAlive(boolean on) {
        keepAlive = on;
        if (!on) runOnUiThread(() -> SessionService.stop(this));
    }

    public void publishWidgets(String payload) {
        WidgetStore.INSTANCE.publish(getApplicationContext(), payload);
        HomeWidget.Companion.broadcastUpdate(getApplicationContext());
    }

    public byte[] sealKey(byte[] plain) {
        try {
            return KeyVault.seal(plain);
        } catch (Exception e) {
            Log.e(TAG, "sealKey", e);
            return null;
        }
    }

    public byte[] openKey(byte[] sealed) {
        try {
            return KeyVault.open(sealed);
        } catch (Exception e) {
            Log.e(TAG, "openKey", e);
            return null;
        }
    }

    public byte[] legacyStoreKey() {
        return LegacyVault.storeKey(this);
    }

    // ---- lifecycle

    @Override
    protected void onPause() {
        // Before onStop: Android 12+ refuses a foreground service started
        // from the background.
        if (keepAlive) SessionService.start(this);
        super.onPause();
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode == SAVE_REQUEST) {
            byte[] bytes = pendingSave;
            pendingSave = null;
            if (resultCode != Activity.RESULT_OK || data == null || data.getData() == null || bytes == null) return;
            try (OutputStream out = getContentResolver().openOutputStream(data.getData())) {
                if (out != null) out.write(bytes);
            } catch (Exception e) {
                Log.e(TAG, "save", e);
                Toast.makeText(this, e.getMessage(), Toast.LENGTH_LONG).show();
            }
            return;
        }
        if (pickRequests.remove(requestCode) == null) return;
        if (resultCode != Activity.RESULT_OK || data == null || data.getData() == null) return;
        Uri uri = data.getData();
        new Thread(() -> {
            try {
                String name = displayName(uri);
                long size = size(uri);
                if (size > MAX_PICK) {
                    runOnUiThread(() -> Toast.makeText(this, "File too large", Toast.LENGTH_LONG).show());
                    return;
                }
                try (InputStream in = getContentResolver().openInputStream(uri)) {
                    if (in == null) return;
                    ByteArrayOutputStream buf = new ByteArrayOutputStream();
                    byte[] chunk = new byte[64 * 1024];
                    int n;
                    while ((n = in.read(chunk)) > 0) buf.write(chunk, 0, n);
                    nativeFilePicked(requestCode, name, buf.toByteArray());
                }
            } catch (Exception e) {
                Log.e(TAG, "pick", e);
            }
        }, "pick").start();
    }

    private String displayName(Uri uri) {
        try (Cursor c = getContentResolver().query(uri, new String[] { OpenableColumns.DISPLAY_NAME }, null, null, null)) {
            if (c != null && c.moveToFirst()) return c.getString(0);
        } catch (Exception ignored) {
        }
        String last = uri.getLastPathSegment();
        return last == null ? "file" : last;
    }

    private long size(Uri uri) {
        try (Cursor c = getContentResolver().query(uri, new String[] { OpenableColumns.SIZE }, null, null, null)) {
            if (c != null && c.moveToFirst() && !c.isNull(0)) return c.getLong(0);
        } catch (Exception ignored) {
        }
        return 0;
    }
}
