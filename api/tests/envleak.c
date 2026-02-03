#include <stdio.h>
#include <stdlib.h>
int main() {
    char *flag = getenv("FLAG");
    if (flag) printf("%s\n", flag);
    return 0;
}
