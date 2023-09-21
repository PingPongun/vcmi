cargo ndk -t arm64-v8a -o ../android_v2/app/src/main/jniLibs/ build
cd ../android_v2
./gradlew build
./gradlew installDebug
adb shell am start -n eu.vcmi.vcmi/.MainActivity
