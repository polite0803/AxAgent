@echo off
cd /d "d:\OneManager\AxAgent"
node scripts\_i18n_audit_fix.mjs > scripts\_i18n_audit_output.txt 2>&1
echo Exit code: %ERRORLEVEL% >> scripts\_i18n_audit_output.txt