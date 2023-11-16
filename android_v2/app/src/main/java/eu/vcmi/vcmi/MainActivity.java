package eu.vcmi.vcmi;

import androidx.appcompat.app.AppCompatActivity;
import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsCompat;
import androidx.core.view.WindowInsetsControllerCompat;
import androidx.documentfile.provider.DocumentFile;

import com.google.androidgamesdk.GameActivity;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.AsyncTask;
import android.os.Bundle;
import android.content.pm.PackageManager;
import android.os.Build.VERSION;
import android.os.Build.VERSION_CODES;
import android.os.Environment;
import android.provider.DocumentsContract;
import android.view.View;
import android.view.WindowManager;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.List;

public class MainActivity extends GameActivity {

    static {
        // Load the STL first to workaround issues on old Android versions:
        // "if your app targets a version of Android earlier than Android 4.3
        // (Android API level 18),
        // and you use libc++_shared.so, you must load the shared library before any other
        // library that depends on it."
        // See https://developer.android.com/ndk/guides/cpp-support#shared_runtimes
        //System.loadLibrary("c++_shared");

        // Load the native library.
        // The name "android-game" depends on your CMake configuration, must be
        // consistent here and inside AndroidManifect.xml
        System.loadLibrary("vcmilauncherv2lib");
    }

    private void hideSystemUI() {
        // This will put the game behind any cutouts and waterfalls on devices which have
        // them, so the corresponding insets will be non-zero.
        if (VERSION.SDK_INT >= VERSION_CODES.P) {
            getWindow().getAttributes().layoutInDisplayCutoutMode
                    = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS;
        }
        // From API 30 onwards, this is the recommended way to hide the system UI, rather than
        // using View.setSystemUiVisibility.
        View decorView = getWindow().getDecorView();
        WindowInsetsControllerCompat controller = new WindowInsetsControllerCompat(getWindow(),
                decorView);
        controller.hide(WindowInsetsCompat.Type.systemBars());
        controller.hide(WindowInsetsCompat.Type.displayCutout());
        controller.setSystemBarsBehavior(
                WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        // When true, the app will fit inside any system UI windows.
        // When false, we render behind any system UI windows.
        WindowCompat.setDecorFitsSystemWindows(getWindow(), false);
        hideSystemUI();
        // You can set IME fields here or in native code using GameActivity_setImeEditorInfoFields.
        // We set the fields in native_engine.cpp.
        // super.setImeEditorInfoFields(InputType.TYPE_CLASS_TEXT,
        //     IME_ACTION_NONE, IME_FLAG_NO_FULLSCREEN );
        super.onCreate(savedInstanceState);
    }

    @Override
    protected void onResume() {
        hideSystemUI();
        super.onResume();
    }

    public boolean isGooglePlayGames() {
        PackageManager pm = getPackageManager();
        return pm.hasSystemFeature("com.google.android.play.feature.HPE_EXPERIENCE");
    }

    public void onLaunchGameBtnPressed() {
        startActivity(new Intent(MainActivity.this, VcmiSDLActivity.class));
    }

    public static final int PICK_EXTERNAL_VCMI_DATA_TO_COPY = 3;

    public void onSelectHoMMDataBtnPressed() {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);

        intent.putExtra(
                DocumentsContract.EXTRA_INITIAL_URI,
                Uri.fromFile(new File(Environment.getExternalStorageDirectory(), "vcmi-data")));

        startActivityForResult(intent, PICK_EXTERNAL_VCMI_DATA_TO_COPY);
    }

    @Override
    public void onActivityResult(int requestCode, int resultCode, Intent resultData) {
        if (requestCode == PICK_EXTERNAL_VCMI_DATA_TO_COPY
                && resultCode == Activity.RESULT_OK) {
            Uri uri;

            if (resultData != null) {
                uri = resultData.getData();

                if (checkDir(uri)) {
                    GetHoMMDirProgress("COPY_START");

                    AsyncCopyData copyTask = new AsyncCopyData(MainActivity.this, uri);
                    copyTask.execute();
                } else {
                    GetHoMMDirProgress("INVALID");
                }

            } else {
                GetHoMMDirProgress("NULL");
            }

            return;
        }
        super.onActivityResult(requestCode, resultCode, resultData);
    }

    public boolean checkDir(Uri uri) {
        DocumentFile sourceDir = DocumentFile.fromTreeUri(MainActivity.this, uri);

        boolean mp3 = false;
        boolean data = false;
        boolean maps = false;

        for (DocumentFile child : sourceDir.listFiles()) {
            if ("maps".equalsIgnoreCase(child.getName())) {
                maps = true;
            }
            if ("mp3".equalsIgnoreCase(child.getName())) {
                mp3 = true;
            }
            if ("data".equalsIgnoreCase(child.getName())) {
                data = true;
            }
        }
        return maps && data && mp3;
    }

    public static native void GetHoMMDirProgress(String progress);

    private class AsyncCopyData extends AsyncTask<String, String, Boolean> {
        private Activity owner;
        private Uri folderToCopy;

        public AsyncCopyData(Activity owner, Uri folderToCopy) {
            this.owner = owner;
            this.folderToCopy = folderToCopy;
        }

        @Override
        protected Boolean doInBackground(final String... params) {
            File targetDir = Storage.getVcmiDataDir(owner);
            DocumentFile sourceDir = DocumentFile.fromTreeUri(owner, folderToCopy);

            ArrayList<String> allowedFolders = new ArrayList<String>();

            allowedFolders.add("Data");
            allowedFolders.add("Mp3");
            allowedFolders.add("Maps");
            allowedFolders.add("Saves");
            allowedFolders.add("Mods");
            allowedFolders.add("config");

            boolean ret = copyDirectory(targetDir, sourceDir, allowedFolders);
            if (ret) {
                GetHoMMDirProgress("COPY_END");
            } else {
                GetHoMMDirProgress("COPY_FAIL");
            }
            return ret;
        }

        private boolean copyDirectory(File targetDir, DocumentFile sourceDir, List<String> allowed) {
            if (!targetDir.exists()) {
                targetDir.mkdir();
            }

            for (DocumentFile child : sourceDir.listFiles()) {
                if (allowed != null) {
                    boolean fileAllowed = false;

                    for (String str : allowed) {
                        if (str.equalsIgnoreCase(child.getName())) {
                            fileAllowed = true;
                            break;
                        }
                    }

                    if (!fileAllowed)
                        continue;
                }

                File exported = new File(targetDir, child.getName());

                if (child.isFile()) {

                    if (!exported.exists()) {
                        try {
                            exported.createNewFile();
                        } catch (IOException e) {
                            return false;
                        }
                    }

                    try (
                            final OutputStream targetStream = new FileOutputStream(exported, false);
                            final InputStream sourceStream = owner.getContentResolver()
                                    .openInputStream(child.getUri())) {
                        copyStream(sourceStream, targetStream);
                    } catch (IOException e) {
                        return false;
                    }
                }

                if (child.isDirectory() && !copyDirectory(exported, child, null)) {
                    return false;
                }
            }

            return true;
        }
    }

    private static final int BUFFER_SIZE = 4096;

    public static void copyStream(InputStream source, OutputStream target) throws IOException {
        final byte[] buffer = new byte[BUFFER_SIZE];
        int read;
        while ((read = source.read(buffer)) != -1) {
            target.write(buffer, 0, read);
        }
    }
}