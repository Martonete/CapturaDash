/** Identificador único para objetos creados en la interfaz. */
export function newId(): string {
  return crypto.randomUUID();
}
