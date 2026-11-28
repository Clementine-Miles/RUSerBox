package tech.lolli.toolbox;

import android.app.NativeActivity;
import android.util.Log;

/** Android window feedback during native surface initialization. */
public final class NativeClientActivity extends NativeActivity {
    // Called by the native activity thread. Window callbacks belong to the UI thread.
    public void reportSurfaceStatus(String status) {
        Log.i("RUSerBox", status);
        runOnUiThread(() -> {
            if (!isDestroyed() && !isFinishing()) {
                setTitle(status);
            }
        });
    }
}
