import { invoke } from "./invoke";
import { ZodSchema } from "zod";

type ValidatedInvokeOptions<T> = {
  command: string;
  args?: Record<string, unknown>;
  schema: ZodSchema<T>;
  strict?: boolean;
};

export async function validatedInvoke<T>({
  command,
  args = {},
  schema,
  strict = false,
}: ValidatedInvokeOptions<T>): Promise<T> {
  const raw = await invoke<T>(command, args);

  const result = schema.safeParse(raw);

  if (result.success) {
    return result.data;
  }

  if (strict) {
    console.error(`[validatedInvoke] Schema validation FAILED for command "${command}":`, result.error);
    throw new Error(`Response validation failed for ${command}: ${result.error.message}`);
  }

  console.warn(
    `[validatedInvoke] Schema validation warning for command "${command}":`,
    result.error.message,
    "Returning raw data as fallback.",
  );
  return raw as T;
}

export function createValidatedInvoker<T>(command: string, schema: ZodSchema<T>, strict = false) {
  return (args?: Record<string, unknown>) =>
    validatedInvoke({ command, args: args ?? {}, schema, strict });
}
