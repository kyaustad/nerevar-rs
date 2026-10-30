import { z } from "zod";

// A synced instance only needs its host: its name, description, TES3MP-NG
// release, game server and settings come from the host.
export const newConnectionSchema = z.object({
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
  remoteHost: "127.0.0.1",
  remoteSyncPort: 25567,
  syncPassword: "",
};
