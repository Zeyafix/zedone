buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:9.3.1")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.2.10")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

apply(from = file("../../../../prototypes/src-tauri/gen/android/tauri-lifecycle.gradle.kts"))



tasks.register("clean").configure {
    delete("build")
}
