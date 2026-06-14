export async function flushStdStreams() {
  await Promise.allSettled(
    [process.stdout, process.stderr]
      .filter(Boolean)
      .map(
        (stream) =>
          new Promise((resolve) => {
            if (stream.writableLength === 0) {
              resolve(undefined);
              return;
            }
            stream.write("", () => resolve(undefined));
          }),
      ),
  );
}

export async function exitSuccess() {
  await flushStdStreams();
  process.exit(0);
}

export async function exitFailure() {
  await flushStdStreams();
  process.exit(1);
}
