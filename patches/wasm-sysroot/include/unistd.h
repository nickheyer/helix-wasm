// unistd.h for wasm32-unknown-unknown
#ifndef _UNISTD_H
#define _UNISTD_H

#include <stddef.h>

typedef int pid_t;
typedef int uid_t;
typedef int gid_t;
typedef long ssize_t;
typedef long off_t;

#define STDIN_FILENO  0
#define STDOUT_FILENO 1
#define STDERR_FILENO 2

ssize_t read(int, void *, size_t);
ssize_t write(int, const void *, size_t);
int close(int);
int dup(int);
int dup2(int, int);
off_t lseek(int, off_t, int);
int isatty(int);
unsigned int sleep(unsigned int);
int usleep(unsigned int);
char *getcwd(char *, size_t);
int chdir(const char *);
int unlink(const char *);
int rmdir(const char *);
int access(int, int);
pid_t getpid(void);

#define F_OK 0
#define R_OK 4
#define W_OK 2
#define X_OK 1

#endif
