import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { InterceptedRequest, DnsEntry } from "../types";

export function useProxy() {
  const [running, setRunning] = useState(false);
  const [requests, setRequests] = useState<InterceptedRequest[]>([]);
  const [dnsQueries, setDnsQueries] = useState<DnsEntry[]>([]);
  const [error, setError] = useState("");

  useEffect(() => {
    const unlisten = listen<InterceptedRequest>("intercepted-request", (event) => {
      setRequests((prev) => [event.payload, ...prev].slice(0, 100));
    });

    const unlistenDns = listen<DnsEntry>("dns-query", (event) => {
      setDnsQueries((prev) => [event.payload, ...prev].slice(0, 50));
    });

    invoke<DnsEntry[]>("get_dns_log")
      .then(setDnsQueries)
      .catch((e) => console.error("Failed to get DNS log:", e));

    return () => {
      unlisten.then((fn) => fn());
      unlistenDns.then((fn) => fn());
    };
  }, []);

  const startProxy = useCallback(async () => {
    try {
      setError("");
      const result = await invoke<string>("start_proxy");
      console.log(result);
      setRunning(true);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  return { running, requests, dnsQueries, error, startProxy, setError };
}
