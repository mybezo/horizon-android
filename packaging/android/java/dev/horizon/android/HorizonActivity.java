package dev.horizon.android;

/** Forward document results and chat navigation to the native game. */
public final class HorizonActivity extends android.app.NativeActivity {
    static { System.loadLibrary("horizon_android"); }
    private final boolean[] chatKeysHeld = new boolean[3];
    private native boolean nativeChatKey(int key);
    private native void nativeDocumentResult(int request, int result, String uri);

    @Override public boolean dispatchKeyEvent(android.view.KeyEvent event) {
        int key = event.getKeyCode();
        int slot;
        switch (key) {
            case android.view.KeyEvent.KEYCODE_TAB: slot = 0; break;
            case android.view.KeyEvent.KEYCODE_DPAD_UP: slot = 1; break;
            case android.view.KeyEvent.KEYCODE_DPAD_DOWN: slot = 2; break;
            default: return super.dispatchKeyEvent(event);
        }
        if (event.getAction() == android.view.KeyEvent.ACTION_DOWN && nativeChatKey(key)) {
            chatKeysHeld[slot] = true;
            return true;
        }
        if (event.getAction() == android.view.KeyEvent.ACTION_UP && chatKeysHeld[slot]) {
            chatKeysHeld[slot] = false;
            return true;
        }
        return super.dispatchKeyEvent(event);
    }

    @Override protected void onActivityResult(int request, int result, android.content.Intent data) {
        super.onActivityResult(request, result, data);
        if (request != 4101) return;
        nativeDocumentResult(request, result,
                data == null || data.getData() == null ? null : data.getData().toString());
    }
}
