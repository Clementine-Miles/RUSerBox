#ifndef FLUTTER_PTY_H_
#define FLUTTER_PTY_H_

#if defined(__linux__) || defined(__GLIBC__) || defined(__GNU__)
#define _GNU_SOURCE /* GNU glibc grantpt() prototypes */
#endif

#include "include/dart_api_dl.h"

typedef struct PtyOptions
{
    int rows;

    int cols;

    char *executable;

    char **arguments;

    char **environment;

    char *working_directory;

    Dart_Port stdout_port;

    Dart_Port exit_port;

    bool ackRead;

} PtyOptions;

typedef struct PtyHandle PtyHandle;

PtyHandle *pty_create(PtyOptions *options);

void pty_write(PtyHandle *handle, char *buffer, int length);

void pty_ack_read(PtyHandle *handle);

int pty_resize(PtyHandle *handle, int rows, int cols);

int pty_getpid(PtyHandle *handle);

char *pty_error(void);

#endif