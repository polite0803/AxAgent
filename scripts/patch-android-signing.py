#!/usr/bin/env python3
"""Patch Tauri Android build.gradle.kts to enable APK signing."""
import re
import sys
import os

def main():
    gradle_path = sys.argv[1] if len(sys.argv) > 1 else "src-tauri/gen/android/app/build.gradle.kts"

    if not os.path.exists(gradle_path):
        print(f"build.gradle.kts not found at {gradle_path}")
        sys.exit(1)

    with open(gradle_path, "r") as f:
        content = f.read()

    if "signingConfigs" in content:
        print("signingConfigs already present, skipping patch")
        sys.exit(0)

    signing_configs = """  signingConfigs {
    create("release") {
      val keystorePropertiesFile = rootProject.file("keystore.properties")
      val keystoreProperties = Properties()
      if (keystorePropertiesFile.exists()) {
        keystoreProperties.load(FileInputStream(keystorePropertiesFile))
      }
      keyAlias = keystoreProperties["keyAlias"] as String
      keyPassword = keystoreProperties["keyPassword"] as String
      storeFile = file(keystoreProperties["storeFile"] as String)
      storePassword = keystoreProperties["storePassword"] as String
    }
  }
"""

    # Insert signingConfigs before buildTypes block
    content = content.replace("  buildTypes {", signing_configs + "  buildTypes {")

    # Add signingConfig to release buildType
    content = re.sub(
        r'(getByName\("release"\) \{[^\n]*\n)',
        r'\1            signingConfig = signingConfigs.getByName("release")\n',
        content
    )

    # Add import if not present
    if "import java.io.FileInputStream" not in content:
        content = "import java.io.FileInputStream\n" + content

    with open(gradle_path, "w") as f:
        f.write(content)

    print(f"Patched {gradle_path} with signing configuration")

if __name__ == "__main__":
    main()
