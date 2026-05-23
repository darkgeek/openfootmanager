import { useState, useRef, type ReactNode, type ReactElement, type MouseEvent } from "react";
import { createPortal } from "react-dom";

interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
}

export default function Tooltip({ content, children }: TooltipProps) {
  const [isVisible, setIsVisible] = useState(false);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const triggerRef = useRef<HTMLElement>(null);
  const timeoutRef = { current: null as ReturnType<typeof setTimeout> | null };

  const updatePosition = () => {
    if (triggerRef.current) {
      const rect = triggerRef.current.getBoundingClientRect();
      setPosition({
        x: rect.left + rect.width / 2,
        y: rect.bottom + 8,
      });
    }
  };

  const handleMouseEnter = () => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    updatePosition();
    setIsVisible(true);
  };

  const handleMouseLeave = () => {
    timeoutRef.current = setTimeout(() => {
      setIsVisible(false);
    }, 100);
  };

  // Clone the child element to add event handlers and ref
  const child = children as ReactElement<Record<string, unknown>>;
  const childProps = child.props || {} as Record<string, unknown>;

  const enhancedChild = (
    <child.type
      {...childProps}
      ref={triggerRef}
      onMouseEnter={(e: MouseEvent) => {
        if (childProps.onMouseEnter) {
          (childProps.onMouseEnter as (e: MouseEvent) => void)(e);
        }
        handleMouseEnter();
      }}
      onMouseLeave={(e: MouseEvent) => {
        if (childProps.onMouseLeave) {
          (childProps.onMouseLeave as (e: MouseEvent) => void)(e);
        }
        handleMouseLeave();
      }}
    />
  );

  return (
    <>
      {enhancedChild}
      {isVisible && createPortal(
        <div
          style={{
            position: "fixed",
            left: `${position.x}px`,
            top: `${position.y}px`,
            transform: "translateX(-50%)",
            zIndex: 9999,
            pointerEvents: "none",
          }}
        >
          <div
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
          </div>
          <div
            style={{
              position: "absolute",
              left: "50%",
              top: "-6px",
              transform: "translateX(-50%)",
              width: 0,
              height: 0,
              borderLeft: "6px solid transparent",
              borderRight: "6px solid transparent",
              borderBottom: "6px solid rgb(11, 15, 35)",
            }}
          />
        </div>,
        document.body
      )}
    </>
  );
}
