import com.android.build.gradle.BaseExtension
import java.security.MessageDigest
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.FileSystemOperations
import org.gradle.api.tasks.Internal
import javax.inject.Inject

abstract class PrepareTauriLifecycle : DefaultTask() {
    @get:InputDirectory abstract val upstream: DirectoryProperty
    @get:InputDirectory abstract val replacement: DirectoryProperty
    @get:OutputDirectory abstract val generated: DirectoryProperty
    @get:Internal abstract val buildRoot: DirectoryProperty
    @get:Inject abstract val files: FileSystemOperations

    @TaskAction fun prepare() {
        val source = upstream.file("app/tauri/plugin/PluginManager.kt").get().asFile.readText().replace("\r\n", "\n")
        val digest = MessageDigest.getInstance("SHA-256").digest(source.toByteArray()).joinToString("") { "%02x".format(it) }
        check(digest == "053958be1e83d8633d26348ded26bbd6d4ab21121e662985194fa3342eef1e95") {
            "Tauri Android source changed; review the BUG-001 lifecycle patch before building"
        }
        check(generated.get().asFile.canonicalFile == buildRoot.get().asFile.resolve("zedone-tauri-lifecycle").canonicalFile)
        files.sync { from(upstream); into(generated) }
        val destination = generated.get().asFile.resolve("app/tauri/plugin")
        destination.mkdirs()
        for (name in listOf("PluginManager.kt", "PluginLifecycle.kt")) {
            replacement.file(name).get().asFile.copyTo(destination.resolve(name), overwrite = true)
        }
    }
}

project(":tauri-android") {
    // Keep this project's patched binaries out of Cargo's shared registry tree.
    layout.buildDirectory.set(rootProject.layout.buildDirectory.dir("zedone-tauri-android-build"))
    plugins.withId("com.android.library") {
        val patched = tasks.register<PrepareTauriLifecycle>("prepareZeDonePluginLifecycle") {
            upstream.set(layout.projectDirectory.dir("src/main/java"))
            replacement.set(rootProject.layout.projectDirectory.dir("../../android-patches"))
            generated.set(rootProject.layout.buildDirectory.dir("zedone-tauri-lifecycle"))
            buildRoot.set(rootProject.layout.buildDirectory)
        }
        extensions.configure<BaseExtension> {
            sourceSets.getByName("main").java.apply {
                setSrcDirs(listOf(patched.flatMap { it.generated }))
            }
        }
        tasks.matching { it.name == "preBuild" }.configureEach { dependsOn(patched) }
    }
}
