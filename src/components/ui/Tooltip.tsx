import { useState, type ReactNode } from "react";

interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
}

export default function Tooltip({ content, children }: TooltipProps) {
  const [isVisible, setIsVisible] = useState(false);
  const timeoutRef = { current: null as ReturnType<typeof setTimeout> | null };

  const handleMouseEnter = () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setIsVisible(true);
  };

  const handleMouseLeave = () => {
    timeoutRef.current = setTimeout(() => {
      setIsVisible(false);
    }, 100);
  };

  return (
    <span
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {children}
      {isVisible && (
        <span
          style={{
            position: "absolute",
            left: "50%",
            top: "100%",
            transform: "translateX(-50%)",
            zIndex: 9999,
          }}
          onMouseEnter={() => {
            if (timeoutRef.current) {
              clearTimeout(timeoutRef.current);
              timeoutRef.current = null;
            }
          }}
          onMouseLeave={handleMouseLeave}
        >
          <span
            className="rounded-lg p-3 max-w-xs whitespace-nowrap"
            style={{
              backgroundColor: "rgb(11, 15, 35)",
              color: "white",
              fontSize: "12px",
              boxShadow: "0 25px 50px -12px rgba(0, 0, 0, 0.25)",
              border: "1px solid rgb(75, 85, 99)",
            }}
          >
            {content}
          </span>
          <span
            style={{
              position: "absolute",
              left: "50%",
              bottom: "-6px",
              transform: "translateX(-50%)",
              width: 0,
              height: 0,
              borderLeft: "6px solid transparent",
              borderRight: "6px solid transparent",
              borderBottom: "6px solid rgb(11, 15, 35)",
            }}
          />
        </span>
      )}
    </span>
  );
}
