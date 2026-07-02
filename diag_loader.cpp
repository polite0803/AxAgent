// diagnostic.cpp - 检查二进制依赖是否能加载
#include <windows.h>
#include <stdio.h>

int main() {
    HMODULE h = LoadLibraryExA("D:\\OneManager\\AxAgent\\src-tauri\\target\\debug\\deps\\axagent_agent-b9e4630f11fcbb16.exe", NULL, DONT_RESOLVE_DLL_REFERENCES);
    if (!h) {
        printf("LoadLibraryExA failed: %lu\n", GetLastError());
        return 1;
    }
    printf("Loaded OK: %p\n", h);
    FreeLibrary(h);
    return 0;
}
