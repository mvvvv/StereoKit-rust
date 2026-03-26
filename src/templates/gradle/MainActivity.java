package ${APPLICATION_ID};

import android.os.Bundle;
import android.util.Log;
import net.stereokit.sk_app.SkAppActivity;

public class MainActivity extends SkAppActivity {
    static {
        //System.loadLibrary("c++_shared");
        System.loadLibrary("openxr_loader");
        System.loadLibrary("${CARGO_LIBNAME}");
    }


    @Override
    protected void onCreate( Bundle savedInstanceState ) {
        Log.d("StereoKitJ", "!!!!onCreate");
        super.onCreate(savedInstanceState);
    }

    @Override
    protected void onResume() {
        super.onResume();
    }
	
	@Override
    protected void onPause() {
	    super.onPause();
    }

    @Override
    protected void onDestroy( ) {
        Log.d("StereoKitJ", "!!!!onDestroy");
        super.onDestroy();
        //Process.killProcess(Process.myPid());
    }
}
