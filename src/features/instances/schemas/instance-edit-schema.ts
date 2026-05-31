import { z } from "zod";

export const instanceEditSchema = z.object({
  name: z
    .string()
    .trim()
    .min(1, "Name is required")
    .max(64, "Name must be at most 64 characters"),
  description: z
    .string()
    .max(500, "Description must be at most 500 characters"),
  host: z
    .string()
    .trim()
    .min(1, "Host is required")
    .max(253, "Host is too long"),
  port: z
    .number()
    .int("Port must be a whole number")
    .min(1, "Port must be at least 1")
    .max(65535, "Port must be at most 65535"),
  password: z.string(),
});

export type InstanceEditFormValues = z.infer<typeof instanceEditSchema>;
