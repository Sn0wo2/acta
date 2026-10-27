import { dlopen, ptr } from "bun:ffi";

export enum LogLevel {
  Error = 0,
  Warn = 1,
  Info = 2,
  Debug = 3,
  Trace = 4,
}

export interface ByteWriter {
  write(bytes: Uint8Array): number | void | Promise<number | void>;
}

export interface LoggerOptions {
  libraryPath: string;
  config?: string | Uint8Array | Record<string, unknown>;
  capacity?: number;
}

export interface Event {
  level: "error" | "warn" | "info" | "debug" | "trace";
  message: string;
  target?: string;
  name?: string;
  module_path?: string;
  file?: string;
  line?: number;
  fields?: Record<string, unknown>;
  parent?: number;
}

export class ActaFfiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ActaFfiError";
  }
}

export const ABI_VERSION = 2;
const DEFAULT_CAPACITY = 64 * 1024;
const DEFAULT_DRAIN_SIZE = 64 * 1024;
const encoder = new TextEncoder();
const statusNames = [
  "success",
  "invalid argument",
  "invalid handle",
  "logger error",
  "internal error",
];

function nativePointerSize(): 4 | 8 {
  switch (process.arch) {
    case "ia32":
    case "arm":
      return 4;
    case "x64":
    case "arm64":
    case "ppc64":
    case "s390x":
    case "riscv64":
      return 8;
    default:
      throw new Error(`Unsupported Bun architecture: ${process.arch}`);
  }
}

const pointerSize = nativePointerSize();
const maxNativeSize = pointerSize === 4 ? 0xffff_ffff : Number.MAX_SAFE_INTEGER;
const maxSliceSize = pointerSize === 4 ? 0x7fff_ffff : Number.MAX_SAFE_INTEGER;

function nativeSizeOutput(): Uint32Array | BigUint64Array {
  return pointerSize === 4 ? new Uint32Array(1) : new BigUint64Array(1);
}

function readNativeSize(output: Uint32Array | BigUint64Array): number {
  const value = output[0]!;
  if (typeof value === "bigint") {
    if (value > BigInt(Number.MAX_SAFE_INTEGER)) {
      throw new RangeError("Native size exceeds JavaScript's safe integer range");
    }
    return Number(value);
  }
  return value;
}

function checkedSize(value: number, name: string, maximum = maxNativeSize): number {
  if (!Number.isSafeInteger(value) || value < 0 || value > maximum) {
    throw new RangeError(`${name} must be a non-negative native-sized integer`);
  }
  return value;
}

function checkedUint32(value: number, name: string): number {
  if (!Number.isInteger(value) || value < 0 || value > 0xffff_ffff) {
    throw new RangeError(`${name} must be an unsigned 32-bit integer`);
  }
  return value;
}

function bytesFor(input: string | Uint8Array): Uint8Array {
  return typeof input === "string" ? encoder.encode(input) : input;
}

function bytePointer(bytes: Uint8Array): number | null {
  return bytes.byteLength === 0 ? null : ptr(bytes);
}

function openLibrary(libraryPath: string) {
  return dlopen(libraryPath, {
    acta_abi_version: { args: [], returns: "u32" },
    acta_version: { args: [], returns: "cstring" },
    acta_last_error: { args: [], returns: "cstring" },
    acta_logger_create: {
      args: ["ptr", "usize", "usize", "ptr"],
      returns: "i32",
    },
    acta_logger_log: {
      args: ["u64", "u32", "ptr", "usize"],
      returns: "i32",
    },
    acta_logger_event: {
      args: ["u64", "ptr", "usize"],
      returns: "i32",
    },
    acta_logger_create_span: {
      args: ["u64", "ptr", "usize", "ptr"],
      returns: "i32",
    },
    acta_logger_record_span: {
      args: ["u64", "u32", "ptr", "usize"],
      returns: "i32",
    },
    acta_logger_close_span: { args: ["u64", "u32"], returns: "i32" },
    acta_logger_drain: {
      args: ["u64", "ptr", "usize", "ptr", "ptr"],
      returns: "i32",
    },
    acta_logger_set_filter: {
      args: ["u64", "ptr", "usize"],
      returns: "i32",
    },
    acta_logger_destroy: { args: ["u64"], returns: "i32" },
  });
}

type NativeLibrary = ReturnType<typeof openLibrary>;

/** Loads the library and returns the ABI version it implements. */
export function abiVersion(libraryPath: string): number {
  const library = openLibrary(libraryPath);
  try {
    return library.symbols.acta_abi_version();
  } finally {
    library.close();
  }
}

/** Loads the library and returns the acta version string. */
export function version(libraryPath: string): string {
  const library = openLibrary(libraryPath);
  try {
    return library.symbols.acta_version() ?? "";
  } finally {
    library.close();
  }
}

function checkStatus(status: number, operation: string, library: NativeLibrary): void {
  if (status === 0) return;
  const description = statusNames[status] ?? `unknown status ${status}`;
  const detail = library.symbols.acta_last_error();
  throw new ActaFfiError(
    status,
    detail ? `${operation} failed (${description}): ${detail}` : `${operation} failed (${description})`,
  );
}

export class Logger {
  #library: NativeLibrary | null;
  #handle: bigint | null;
  #pendingDrain = new Uint8Array(0);
  #drainQueue: Promise<void> = Promise.resolve();
  #drainOperationCount = 0;

  constructor(options: LoggerOptions) {
    const library = openLibrary(options.libraryPath);
    try {
      const abi = library.symbols.acta_abi_version();
      if (abi !== ABI_VERSION) {
        throw new Error(`Unsupported acta-ffi ABI version ${abi}; expected ${ABI_VERSION}`);
      }

      const input = options.config ?? "{}";
      const config = bytesFor(typeof input === "string" || input instanceof Uint8Array ? input : JSON.stringify(input));
      const capacity = checkedSize(options.capacity ?? DEFAULT_CAPACITY, "capacity");
      const outHandle = new BigUint64Array(1);
      checkStatus(
        library.symbols.acta_logger_create(
          bytePointer(config),
          config.byteLength,
          capacity,
          ptr(outHandle),
        ),
        "logger create",
        library,
      );
      if (outHandle[0] === 0n) {
        throw new Error("acta-ffi returned an invalid zero logger handle");
      }

      this.#library = library;
      this.#handle = outHandle[0]!;
    } catch (error) {
      library.close();
      throw error;
    }
  }

  log(level: LogLevel, message: string | Uint8Array): void {
    const { library, handle } = this.#state();
    const bytes = bytesFor(message);
    checkStatus(
      library.symbols.acta_logger_log(handle, level, bytePointer(bytes), bytes.byteLength),
      "logger log",
      library,
    );
  }

  event(event: Event): void {
    const { library, handle } = this.#state();
    const bytes = encoder.encode(JSON.stringify(event));
    checkStatus(
      library.symbols.acta_logger_event(handle, bytePointer(bytes), bytes.byteLength),
      "logger event",
      library,
    );
  }

  createSpan(event: Event): number {
    const { library, handle } = this.#state();
    const bytes = encoder.encode(JSON.stringify(event));
    const outId = new Uint32Array(1);
    checkStatus(
      library.symbols.acta_logger_create_span(
        handle,
        bytePointer(bytes),
        bytes.byteLength,
        ptr(outId),
      ),
      "logger create span",
      library,
    );
    return outId[0]!;
  }

  recordSpan(id: number, fields: Record<string, unknown>): void {
    const { library, handle } = this.#state();
    const bytes = encoder.encode(JSON.stringify(fields));
    checkStatus(
      library.symbols.acta_logger_record_span(
        handle,
        checkedUint32(id, "id"),
        bytePointer(bytes),
        bytes.byteLength,
      ),
      "logger record span",
      library,
    );
  }

  closeSpan(id: number): void {
    const { library, handle } = this.#state();
    checkStatus(
      library.symbols.acta_logger_close_span(handle, checkedUint32(id, "id")),
      "logger close span",
      library,
    );
  }

  drain(maxBytes = DEFAULT_DRAIN_SIZE): Promise<Uint8Array> {
    const capacity = checkedSize(maxBytes, "maxBytes", maxSliceSize);
    return this.#serializeDrain(() => {
      this.#state();
      if (capacity === 0) return new Uint8Array(0);
      this.#fillPendingDrain(capacity);

      const written = Math.min(capacity, this.#pendingDrain.byteLength);
      const output = this.#pendingDrain.slice(0, written);
      this.#pendingDrain = this.#pendingDrain.subarray(written);
      return output;
    });
  }

  drainTo(writer: ByteWriter, maxBytes = DEFAULT_DRAIN_SIZE): Promise<number> {
    const capacity = checkedSize(maxBytes, "maxBytes", maxSliceSize);
    return this.#serializeDrain(async () => {
      this.#state();
      if (capacity === 0) return 0;
      this.#fillPendingDrain(capacity);

      let writtenTotal = 0;
      while (writtenTotal < capacity && this.#pendingDrain.byteLength !== 0) {
        const attempted = this.#pendingDrain
          .subarray(0, Math.min(capacity - writtenTotal, this.#pendingDrain.byteLength))
          .slice();
        const result = await writer.write(attempted);
        const written = result ?? attempted.byteLength;
        if (!Number.isSafeInteger(written) || written < 0 || written > attempted.byteLength) {
          throw new RangeError("ByteWriter.write returned an invalid byte count");
        }

        this.#pendingDrain = this.#pendingDrain.subarray(written);
        writtenTotal += written;
        if (written === 0) throw new Error("Byte writer made no progress");
      }
      return writtenTotal;
    });
  }

  dropped(): bigint {
    const { library, handle } = this.#state();
    const outWritten = nativeSizeOutput();
    const outDropped = new BigUint64Array(1);
    checkStatus(
      library.symbols.acta_logger_drain(handle, null, 0, ptr(outWritten), ptr(outDropped)),
      "logger dropped",
      library,
    );
    return outDropped[0]!;
  }

  async flush(writer: ByteWriter, chunkSize = DEFAULT_DRAIN_SIZE): Promise<number> {
    checkedSize(chunkSize, "chunkSize", maxSliceSize);
    if (chunkSize === 0) throw new RangeError("chunkSize must be positive");
    let total = 0;
    while (true) {
      const written = await this.drainTo(writer, chunkSize);
      total += written;
      if (written < chunkSize) return total;
    }
  }

  setFilter(filter: string | Uint8Array): void {
    const { library, handle } = this.#state();
    const bytes = bytesFor(filter);
    checkStatus(
      library.symbols.acta_logger_set_filter(handle, bytePointer(bytes), bytes.byteLength),
      "logger set filter",
      library,
    );
  }

  close(): void {
    const library = this.#library;
    const handle = this.#handle;
    if (!library || handle === null) return;
    if (this.#drainOperationCount !== 0) {
      throw new Error("Cannot close Logger while drain operations are pending");
    }
    if (this.#pendingDrain.byteLength !== 0) {
      throw new Error("Cannot close Logger while drained bytes remain pending");
    }

    checkStatus(library.symbols.acta_logger_destroy(handle), "logger destroy", library);
    this.#library = null;
    this.#handle = null;
    library.close();
  }

  #state(): { library: NativeLibrary; handle: bigint } {
    if (!this.#library || this.#handle === null) {
      throw new Error("Logger is closed");
    }
    return { library: this.#library, handle: this.#handle };
  }

  #serializeDrain<T>(operation: () => T | Promise<T>): Promise<T> {
    this.#drainOperationCount += 1;
    const result = this.#drainQueue.then(operation);
    this.#drainQueue = result.then(
      () => {
        this.#drainOperationCount -= 1;
      },
      () => {
        this.#drainOperationCount -= 1;
      },
    );
    return result;
  }

  #fillPendingDrain(maxBytes: number): void {
    const existingBytes = this.#pendingDrain.byteLength;
    if (existingBytes >= maxBytes) return;

    const { library, handle } = this.#state();
    const additionalCapacity = maxBytes - existingBytes;
    const combined = new Uint8Array(existingBytes + additionalCapacity);
    combined.set(this.#pendingDrain);

    const output = combined.subarray(existingBytes);
    const outWritten = nativeSizeOutput();
    const outDropped = new BigUint64Array(1);
    checkStatus(
      library.symbols.acta_logger_drain(
        handle,
        ptr(output),
        output.byteLength,
        ptr(outWritten),
        ptr(outDropped),
      ),
      "logger drain",
      library,
    );
    const written = readNativeSize(outWritten);
    if (written > output.byteLength) {
      throw new Error("acta-ffi returned a drain length larger than the output buffer");
    }
    this.#pendingDrain = combined.subarray(0, existingBytes + written);
  }
}
