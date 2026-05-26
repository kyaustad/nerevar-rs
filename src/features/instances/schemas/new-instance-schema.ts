import { z } from "zod";

export const newInstanceSchema = z.object({
  releaseId: z.string().min(1, "Select a TES3MP release"),
  instanceName: z
    .string()
    .trim()
    .min(1, "Instance name is required")
    .max(64, "Instance name must be at most 64 characters"),
  instanceDescription: z
    .string()
    .max(500, "Description must be at most 500 characters"),
  instanceRootPath: z.string().min(3, "Instance root path is required"),
  instanceDataDir: z.string().min(3, "Instance data directory is required"),
  serverHostName: z
    .string()
    .trim()
    .min(1, "Server host name is required")
    .max(128, "Server host name must be at most 128 characters"),
  maxPlayers: z
    .number()
    .int("Max players must be a whole number")
    .min(1, "Max players must be at least 1")
    .max(1000, "Max players must be at most 1000"),
  serverPort: z
    .number()
    .int("Server port must be a whole number")
    .min(1, "Server port must be at least 1")
    .max(65535, "Server port must be at most 65535"),
  password: z.string(),
  masterServerEnabled: z.boolean(),
});

export type NewInstanceFormValues = z.infer<typeof newInstanceSchema>;

export const newInstanceDefaultValues: NewInstanceFormValues = {
  releaseId: "",
  instanceName: "",
  instanceDescription: "",
  instanceRootPath: "",
  instanceDataDir: "",
  serverHostName: "A Nerevar Server",
  maxPlayers: 64,
  serverPort: 25565,
  password: "",
  masterServerEnabled: true,
};
