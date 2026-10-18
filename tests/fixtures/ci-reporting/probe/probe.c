/** @file Deliberate null dereference for the temporary reporting probe. */
int Ci_Probe(void);
int Ci_Probe(void)
{
    int *pointer = 0;
    return *pointer;
}
