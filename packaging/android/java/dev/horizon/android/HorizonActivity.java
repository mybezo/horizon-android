package dev.horizon.android;

/** Forward document results and chat Tab to the native game. */
public final class HorizonActivity extends android.app.NativeActivity {
    static { System.loadLibrary("horizon_android"); }
    private boolean chatTabHeld;
    private native boolean nativeChatTab();
    private native void nativeDocumentResult(int request, int result, String uri);

    @Override public boolean dispatchKeyEvent(android.view.KeyEvent event) {
        if (event.getKeyCode() == android.view.KeyEvent.KEYCODE_TAB) {
            if (event.getAction() == android.view.KeyEvent.ACTION_DOWN
                    && nativeChatTab()) {
                chatTabHeld = true;
                return true;
            }
            if (event.getAction() == android.view.KeyEvent.ACTION_UP && chatTabHeld) {
                chatTabHeld = false;
                return true;
            }
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
