const RETRYABLE_CODES = new Set(['ERR_NETWORK', 'ECONNABORTED', 'ETIMEDOUT'])

function isRetryableError(error: unknown): boolean {
  if (typeof error !== 'object' || error == null) return false
  const code = (error as { code?: string }).code
  return code != null && RETRYABLE_CODES.has(code)
}

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

export async function retryAsync<T>(
  task: () => Promise<T>,
  options?: { retries?: number; delayMs?: number },
): Promise<T> {
  const retries = options?.retries ?? 2
  const delayMs = options?.delayMs ?? 600
  let lastError: unknown

  for (let attempt = 0; attempt <= retries; attempt += 1) {
    try {
      return await task()
    } catch (error) {
      lastError = error
      if (attempt >= retries || !isRetryableError(error)) {
        throw error
      }
      await delay(delayMs * (attempt + 1))
    }
  }

  throw lastError
}
