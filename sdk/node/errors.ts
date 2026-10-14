/** Typed errors for the `smol` SDK.
 *
 *  The native addon reports errors as `Error` objects whose message is
 *  prefixed with a bracketed code, e.g. `"[KVM_UNAVAILABLE] …"`. We parse that
 *  back into a typed hierarchy so callers can branch on `err.code` /`instanceof`.
 */

/** What the cloud API said about a failed request, beyond its message. */
export interface CloudErrorDetail {
  /** HTTP status of the response. */
  status: number;
  /** The server's stable problem code, e.g. `machine_interrupted`. */
  problemCode?: string | undefined;
  /** Whether sending the same request again can succeed. */
  retryable: boolean;
  /** How long the server asked callers to wait before retrying (ms). */
  retryAfterMs?: number | undefined;
}

/** The stable codes a `SmolError` carries, to branch on instead of matching
 *  message text (which may change). Unknown codes from a newer engine or
 *  control plane still type-check as plain strings.
 *
 *  - `NOT_FOUND`: no such machine, file (`readFile` of a missing path),
 *    image or checkpoint.
 *  - `INTERRUPTED`: a `stop()` or `delete()` cut off this command or file
 *    transfer (local). `MACHINE_INTERRUPTED`: the same, on the cloud.
 *  - `CONFLICT`: the machine's state does not allow it now (it is being
 *    stopped or deleted, it has dependent branches, ...). `INVALID_STATE`:
 *    the same, from an older engine path.
 *  - `INVALID_CONFIG`: a usage error in the arguments. `CONFIG_ERROR`: the
 *    engine refused a configuration value. `NOT_SUPPORTED`: not on this
 *    target.
 *  - `KVM_UNAVAILABLE`, `HYPERVISOR_UNAVAILABLE`: this host cannot run
 *    machines. `STORAGE_ERROR`, `MOUNT_ERROR`: disk or mount failures.
 *  - `COMMAND_FAILED`: a command exited non-zero (`assertSuccess()`).
 *  - `TIMEOUT`, `CONNECTION`, `UNAUTHORIZED`: cloud transport failures.
 *  - `SMOLVM_ERROR`: anything not classified above. */
export type SmolErrorCode =
  | "NOT_FOUND"
  | "INTERRUPTED"
  | "MACHINE_INTERRUPTED"
  | "CONFLICT"
  | "INVALID_STATE"
  | "FORBIDDEN"
  | "INVALID_CONFIG"
  | "CONFIG_ERROR"
  | "NOT_SUPPORTED"
  | "KVM_UNAVAILABLE"
  | "HYPERVISOR_UNAVAILABLE"
  | "STORAGE_ERROR"
  | "MOUNT_ERROR"
  | "COMMAND_FAILED"
  | "TIMEOUT"
  | "CONNECTION"
  | "UNAUTHORIZED"
  | "SMOLVM_ERROR"
  | (string & {});

export class SmolError extends Error {
  readonly code: SmolErrorCode;
  /** HTTP status, for an error the cloud API returned. */
  readonly status?: number | undefined;
  /** The server's stable problem code, for an error the cloud API returned. */
  readonly problemCode?: string | undefined;
  /** Whether sending the same request again can succeed. A command whose
   *  machine went away (`MACHINE_INTERRUPTED`) is retryable, but it may have
   *  partly run, so the SDK leaves that retry to the caller. */
  readonly retryable: boolean;
  /** How long the server asked callers to wait before retrying (ms). */
  readonly retryAfterMs?: number | undefined;
  constructor(code: SmolErrorCode, message: string, detail?: CloudErrorDetail) {
    super(message);
    this.name = new.target.name;
    this.code = code;
    this.status = detail?.status;
    this.problemCode = detail?.problemCode;
    this.retryable = detail?.retryable ?? false;
    this.retryAfterMs = detail?.retryAfterMs;
  }
}

/** The active backend can't serve this operation (e.g. volumes on local). */
export class NotSupportedError extends SmolError {
  constructor(message: string) {
    super('NOT_SUPPORTED', message);
  }
}

/** A required configuration value is missing or invalid (a usage error). */
export class InvalidConfigError extends SmolError {
  constructor(message: string) {
    super('INVALID_CONFIG', message);
  }
}

/** A command ran but exited non-zero (raised by `ExecResult.assertSuccess()`). */
export class ExecutionError extends SmolError {
  readonly exitCode: number;
  readonly stdout: string;
  readonly stderr: string;
  constructor(exitCode: number, stdout: string, stderr: string) {
    super('COMMAND_FAILED', `Command exited with code ${exitCode}`);
    this.exitCode = exitCode;
    this.stdout = stdout;
    this.stderr = stderr;
  }
}

const BRACKETED = /^\[([A-Z_]+)\]\s*(.*)$/s;

/** Convert any error thrown by the native addon into a typed `SmolError`. */
export function wrapNativeError(err: unknown): SmolError {
  if (err instanceof SmolError) return err;
  const message = err instanceof Error ? err.message : String(err);
  const m = BRACKETED.exec(message);
  if (m) return new SmolError(m[1], m[2]);
  return new SmolError('SMOLVM_ERROR', message);
}
