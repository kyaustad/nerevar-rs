import { ArrowLeft, Scroll } from "lucide-react";
import { motion, useReducedMotion } from "motion/react";
import { useLocation } from "wouter";
import { Button } from "../ui/button";

const EASE = [0.22, 1, 0.36, 1] as const;

export function NerevarHeader({
  title,
  subtitle,
}: {
  title: string;
  subtitle: string;
}) {
  const reduceMotion = useReducedMotion();
  const [pathname, navigate] = useLocation();

  return (
    <motion.header
      className="mb-8 flex flex-col items-center text-center relative"
      initial={reduceMotion ? false : { opacity: 0, y: -12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: EASE }}
    >
      {pathname !== "/" && (
        <Button
          type="button"
          variant="outline"
          className="absolute left-10 top-2 font-display text-xs tracking-[0.2em] uppercase text-foreground/70 hover:text-accent"
          onClick={() => navigate("/")}
        >
          <ArrowLeft className="size-4" />
          Home
        </Button>
      )}
      <div className="mb-4 flex size-14 items-center justify-center rounded-xl bg-gradient-to-br from-accent/80 to-accent/40 glow-gold">
        <Scroll className="size-7 text-accent-foreground" strokeWidth={1.5} />
      </div>
      <h1 className="font-display text-3xl font-bold tracking-[0.12em] text-gradient-gold md:text-4xl">
        {title}
      </h1>
      <p className="mt-1 font-display text-[0.85rem] tracking-[0.3em] text-foreground/65 uppercase">
        {subtitle}
      </p>
      <div className="morrowind-divider mt-5 w-40" />
    </motion.header>
  );
}
