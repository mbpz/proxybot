import { useSslBypass } from "../../stores/sslBypassStore";

export function FridaStatus() {
  const store = useSslBypass();
  return (
    <div className="card mb-4">
      <h3 className="card-title text-base mb-2">Prerequisites</h3>
      <div className="text-sm space-y-1">
        <div>
          <span className="font-medium">Java:</span>{" "}
          {store.javaInstalled ? (
            <span className="text-accent-green">installed (required for APK patching)</span>
          ) : (
            <span className="text-accent-red">not installed (required for APK patching)</span>
          )}
        </div>
        <div>
          <span className="font-medium">ADB:</span>{" "}
          {store.adbInstalled ? (
            <span className="text-accent-green">installed (required for device connection)</span>
          ) : (
            <span className="text-accent-red">not installed (required for device connection)</span>
          )}
        </div>
      </div>
      <button
        onClick={store.checkPrerequisites}
        className="btn btn-sm btn-ghost mt-3"
      >
        Recheck
      </button>
    </div>
  );
}