@echo off
setlocal enabledelayedexpansion

:: Cross-platform target installation and build verification script for Horizon-Sockets
:: Windows Batch version

:: Parse command line arguments
set INSTALL_ONLY=0
set CHECK_ONLY=0
set VERBOSE=0

:parse_args
if "%1"=="--install-only" (
    set INSTALL_ONLY=1
    shift
    goto parse_args
)
if "%1"=="--check-only" (
    set CHECK_ONLY=1
    shift
    goto parse_args
)
if "%1"=="--verbose" (
    set VERBOSE=1
    shift
    goto parse_args
)
if "%1"=="-v" (
    set VERBOSE=1
    shift
    goto parse_args
)
if "%1"=="--help" (
    echo Usage: %0 [OPTIONS]
    echo.
    echo OPTIONS:
    echo   --install-only    Only install targets, don't run builds
    echo   --check-only      Only run build checks, don't install targets
    echo   --verbose, -v     Show detailed output
    echo   --help            Show this help message
    echo.
    echo Supported targets:
    echo   x86_64-pc-windows-msvc
    echo   x86_64-apple-darwin
    echo   aarch64-apple-darwin
    echo   x86_64-unknown-linux-gnu
    echo   x86_64-unknown-linux-musl
    echo   aarch64-unknown-linux-gnu
    echo   x86_64-unknown-freebsd
    echo   x86_64-unknown-netbsd
    echo   x86_64-unknown-openbsd
    exit /b 0
)
if not "%1"=="" (
    echo Unknown option: %1
    echo Use --help for usage information
    exit /b 1
)

:: Supported targets
set TARGETS=x86_64-pc-windows-msvc x86_64-apple-darwin aarch64-apple-darwin x86_64-unknown-linux-gnu x86_64-unknown-linux-musl aarch64-unknown-linux-gnu x86_64-unknown-freebsd x86_64-unknown-netbsd x86_64-unknown-openbsd

:: Feature configurations
set CONFIG_NAMES=default full mio-only
set CONFIG_default=
set CONFIG_full=--features full
set CONFIG_mio-only=--no-default-features --features mio-runtime

:: Result tracking
set TOTAL_TESTS=0
set PASSED_TESTS=0
set FAILED_TESTS=0

echo 🚀 Horizon-Sockets Cross-Platform Build Checker
echo =================================================

:: Install targets function
if %CHECK_ONLY%==0 (
    echo.
    echo 📦 Installing Rust targets...
    
    for %%t in (%TARGETS%) do (
        echo   Installing %%t...
        rustup target add %%t >nul 2>&1
        if !errorlevel!==0 (
            echo     ✅ Installed
        ) else (
            echo     ❌ Failed to install
        )
    )
)

:: Build testing function
if %INSTALL_ONLY%==0 (
    echo.
    echo 🔨 Testing builds across targets and feature configurations...
    
    set current_test=0
    for %%c in (%CONFIG_NAMES%) do (
        set /a TOTAL_TESTS+=9
    )
    
    for %%t in (%TARGETS%) do (
        echo.
        echo 📋 Target: %%t
        
        for %%c in (%CONFIG_NAMES%) do (
            set /a current_test+=1
            set /a progress=!current_test!*100/!TOTAL_TESTS!
            
            echo   [!progress!%%] Testing %%c...
            
            :: Get the features for this config
            call set features=%%CONFIG_%%c%%
            
            :: Run the build command
            if %VERBOSE%==1 (
                echo     Command: cargo build --target %%t !features!
            )
            
            cargo build --target %%t !features! >nul 2>&1
            if !errorlevel!==0 (
                echo     ✅ PASS
                set /a PASSED_TESTS+=1
            ) else (
                echo     ❌ FAIL
                set /a FAILED_TESTS+=1
                if %VERBOSE%==1 (
                    echo     Running again with output for debugging...
                    cargo build --target %%t !features!
                )
            )
        )
    )
    
    :: Show summary
    echo.
    echo 📊 BUILD SUMMARY
    echo =================
    echo Total Tests: !TOTAL_TESTS!
    echo Passed: !PASSED_TESTS!
    echo Failed: !FAILED_TESTS!
    
    if !FAILED_TESTS!==0 (
        set /a success_rate=100
        echo Success Rate: 100%%
        echo.
        echo 🎉 All tests passed!
        exit /b 0
    ) else (
        set /a success_rate=!PASSED_TESTS!*100/!TOTAL_TESTS!
        echo Success Rate: !success_rate!%%
        echo.
        echo ⚠️  Some tests failed. Run with --verbose for more details.
        exit /b 1
    )
)