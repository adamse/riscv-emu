#include <stdio.h>

void
main(int argc, char** argv) {
    printf("hello world!: %d\n", argc);
    printf("hello world!: %d %s\n", argc, argv[0]);
}
