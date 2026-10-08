#ifndef HOST_STORAGE_FAULTS_H
#define HOST_STORAGE_FAULTS_H
#include <stdio.h>
FILE *Test_Open(const char *, const char *);
int Test_Close(FILE *);
#define fopen Test_Open
#define fclose Test_Close
#endif
