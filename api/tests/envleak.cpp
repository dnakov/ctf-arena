#include <iostream>
#include <cstdlib>
int main() {
    if (char* flag = std::getenv("FLAG"))
        std::cout << flag << std::endl;
}
