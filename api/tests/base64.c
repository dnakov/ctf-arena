#include <stdio.h>
#include <string.h>
static const char b64[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
int idx(char c) { char *p = strchr(b64, c); return p ? p - b64 : 0; }
int main() {
    char buf[65536];
    int n = fread(buf, 1, sizeof(buf), stdin);
    while (n > 0 && (buf[n-1] == '\n' || buf[n-1] == '\r')) n--;
    for (int i = 0; i < n; i += 4) {
        int a = idx(buf[i]), b = idx(buf[i+1]);
        int c = idx(buf[i+2]), d = idx(buf[i+3]);
        putchar((a << 2) | (b >> 4));
        if (buf[i+2] != '=') putchar(((b & 0xf) << 4) | (c >> 2));
        if (buf[i+3] != '=') putchar(((c & 0x3) << 6) | d);
    }
    return 0;
}
