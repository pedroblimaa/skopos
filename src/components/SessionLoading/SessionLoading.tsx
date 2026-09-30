import "./SessionLoading.css";

export function SessionLoading({ label }: { label: string }) {
  return (
    <div className="session-loading" role="status">
      <span className="session-loading-spinner" aria-hidden="true" />
      <p>{label}</p>
    </div>
  );
}
