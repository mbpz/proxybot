import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CaMetadata } from "../types";

export function useCa() {
  const [caMetadata, setCaMetadata] = useState<CaMetadata | null>(null);

  useEffect(() => {
    invoke<CaMetadata | null>("get_ca_metadata")
      .then(setCaMetadata)
      .catch(console.error);
  }, []);

  const downloadCaCert = useCallback(async (onError: (msg: string) => void) => {
    try {
      const caPem = await invoke<string>("get_ca_cert_pem");
      await navigator.clipboard.writeText(caPem);
      alert("CA certificate copied to clipboard. Paste it to a file with .pem extension.");
    } catch (e) {
      onError(String(e));
    }
  }, []);

  return { caMetadata, downloadCaCert };
}
