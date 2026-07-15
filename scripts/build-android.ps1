# SPDX-License-Identifier: MPL-2.0

param(
    [string]$SdkPath,
    [string]$NdkVersion = "29.0.13846066",
    [string]$Proxy
)

$ErrorActionPreference = "Stop"
$repository = Split-Path -Parent $PSScriptRoot

if ([string]::IsNullOrWhiteSpace($SdkPath)) {
    if (-not [string]::IsNullOrWhiteSpace($env:ANDROID_HOME)) {
        $SdkPath = $env:ANDROID_HOME
    } else {
        $SdkPath = Join-Path $repository ".local\android-sdk"
    }
}

if (-not (Test-Path -LiteralPath $SdkPath -PathType Container)) {
    throw "Android SDK not found at '$SdkPath'. See design/android-target.md for setup instructions."
}

$SdkPath = (Resolve-Path -LiteralPath $SdkPath).Path
$NdkPath = Join-Path $SdkPath "ndk\$NdkVersion"
if (-not (Test-Path -LiteralPath $NdkPath -PathType Container)) {
    throw "Android NDK $NdkVersion not found at '$NdkPath'."
}

$env:ANDROID_HOME = $SdkPath
$env:ANDROID_SDK_ROOT = $SdkPath
$env:NDK_HOME = $NdkPath

if (-not [string]::IsNullOrWhiteSpace($Proxy)) {
    $proxyUri = [Uri]$Proxy
    $env:HTTP_PROXY = $Proxy
    $env:HTTPS_PROXY = $Proxy
    $gradleProxy = "-Dhttp.proxyHost=$($proxyUri.Host) -Dhttp.proxyPort=$($proxyUri.Port) -Dhttps.proxyHost=$($proxyUri.Host) -Dhttps.proxyPort=$($proxyUri.Port)"
    $env:GRADLE_OPTS = "$($env:GRADLE_OPTS) $gradleProxy -Dorg.gradle.internal.http.connectionTimeout=120000 -Dorg.gradle.internal.http.socketTimeout=120000".Trim()
}

Push-Location (Join-Path $repository "web")
try {
    npm run android:build:debug
    if ($LASTEXITCODE -ne 0) {
        throw "Android build failed with exit code $LASTEXITCODE."
    }
} finally {
    Pop-Location
}
