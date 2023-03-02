#include <stdio.h>
#include <sys/stat.h>

struct  kernel_stat
{
  unsigned long long st_dev;
  unsigned long long st_ino;
  unsigned int st_mode;
  unsigned int st_nlink;
  unsigned int st_uid;
  unsigned int st_gid;
  unsigned long long st_rdev;
  unsigned long long __pad1;
  long long st_size;
  int st_blksize;
  int __pad2;
  long long st_blocks;
  struct timespec st_atim;
  struct timespec st_mtim;
  struct timespec st_ctim;
  int __glibc_reserved[2];
};

int
main(int argc, char** argv) {
    printf("struct kernel_stat size: %d\n", sizeof(struct kernel_stat));
    printf("struct stat size: %d\n", sizeof(struct stat));
    printf("hello world!: %d\n", argc);
    printf("hello world!: %d %s\n", argc, argv[0]);

    return 0;
}
