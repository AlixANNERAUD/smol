/** Local image source regression; set SMOL_SDK_TEST_IMAGE_ARCHIVE to boot a saved image. */
import assert from 'node:assert/strict';
import { Machine } from '../index';

async function main(): Promise<void> {
  const missing = `/tmp/smol-sdk-missing-image-${process.pid}.tar`;
  await assert.rejects(
    Machine.create({ image: missing, network: false }, { target: 'local' }),
    (error: unknown) => error instanceof Error && error.message.includes(`cannot read archive ${missing}`),
  );

  const archive = process.env.SMOL_SDK_TEST_IMAGE_ARCHIVE;
  if (!archive) {
    console.log('Missing archive diagnosis passed; set SMOL_SDK_TEST_IMAGE_ARCHIVE for a live VM');
    return;
  }
  const machine = await Machine.create(
    { name: `sdk-local-archive-${process.pid}`, image: archive, network: false },
    { target: 'local', handleSignals: false },
  );
  try {
    const result = await machine.exec(['sh', '-c', 'printf local-archive-ok']);
    assert.equal(result.exitCode, 0, result.stderr);
    assert.equal(result.stdout, 'local-archive-ok');
    console.log('Saved image archive booted without a registry pull');
  } finally {
    await machine.delete();
  }
}

main().catch((error: unknown) => { console.error(error); process.exitCode = 1; });
