export declare class Logger {
  constructor(config?: any | undefined | null, capacity?: number | undefined | null)
  log(level: number, message: string): void
  drain(maxBytes: number): Buffer
  event(event: any): void
  createSpan(event: any): number
  recordSpan(id: number, fields: any): void
  closeSpan(id: number): void
  dropped(): bigint
  setFilter(filter: string): void
}
