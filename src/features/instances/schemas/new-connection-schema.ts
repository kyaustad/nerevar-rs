import { z } from "zod";

export const newConnectionSchema = z.object({
  releaseId: z.string().min(1, "Select a TES3MP release"),
  connectionName: z
    .string()
    .trim()
    .min(1, "Connection name is required")
    .max(64, "Name must be at most 64 characters"),
  connectionDescription: z
    .string()
    .max(500, "Description must be at most 500 characters"),
  instanceRootPath: z.string().min(3, "Instance root path is required"),
  instanceDataDir: z.string().min(3, "Data directory is required"),
  remoteHost: z
    .string()
    .trim()
    .min(1, "Remote host is required")
    .max(253, "Host is too long"),
  remoteSyncPort: z
    .number()
    .int("Port must be a whole number")
    .min(1, "Port must be at least 1")
    .max(65535, "Port must be at most 65535"),
  syncPassword: z.string(),
});

export type NewConnectionFormValues = z.infer<typeof newConnectionSchema>;

export const newConnectionDefaultValues: NewConnectionFormValues = {
  releaseId: "",
  connectionName: "",
  connectionDescription: "",
  instanceRootPath: "",
  instanceDataDir: "",
  remoteHost: "127.0.0.1",
  remoteSyncPort: 25567,
  syncPassword: "",
};
