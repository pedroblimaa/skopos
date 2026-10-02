import icon from "../../assets/icon-64.png";
import "./AppIcon.css";

export function AppIcon({ size = 28 }: { size?: number }) {
  return (
    <img className="app-icon" src={icon} width={size} height={size} alt="" draggable={false} />
  );
}
