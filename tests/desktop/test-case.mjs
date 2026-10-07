import test from 'node:test';

// Keep the scenario's fail-fast cleanup while reporting each case through Node.
export async function runCase(name, body) {
  let failure;
  let result;
  await test(name, async () => {
    try {
      result = await body();
    } catch (error) {
      failure = error;
      throw error;
    }
  });
  if (failure) throw failure;
  return result;
}
