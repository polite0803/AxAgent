@echo off
setlocal
cd /d "d:\OneManager\AxAgent"
node scripts\i18n-full-sync.mjs 2>&1
echo DONE