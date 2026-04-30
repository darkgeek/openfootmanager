import { useState, type ReactNode, type MouseEvent, type ReactElement } from "react";
import { cloneElement, isValidElement } from "react";

interface TooltipProps {
  content: ReactNode;
  children: ReactNode;
}

interface TooltipChildProps {
  onMouseEnter?: (e: MouseEvent<HTMLElement>) => void;
  onMouseLeave?: (e: MouseEvent<HTMLElement>) => void;
  onMouseMove?: (e: MouseEvent<HTMLElement>) => void;
}

export default function Tooltip({ content, children }: TooltipProps) {
  const [isVisible, setIsVisible] = useState(false);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const timeoutRef = { current: null as ReturnType<typeof setTimeout> | null };

  const handleMouseMove = (e: MouseEvent<HTMLElement>) => {
    // Use mouse move to track actual position
    setPosition({
      x: e.clientX,
      y: e.clientY + 16,
    });
  };

  const handleMouseEnter = (e: MouseEvent<HTMLElement>) => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    // Get initial position from mouse enter
    setPosition({
      x: e.clientX,
      y: e.clientY + 16,
    });
    setIsVisible(true);
  };

  const handleMouseLeave = () => {
    timeoutRef.current = setTimeout(() => {
      setIsVisible(false);
    }, 100);
  };

  if (!isValidElement(children)) {
    return <>{children}</>;
  }

  const child = children as ReactElement<TooltipChildProps>;

  const clonedChild = cloneElement(child, {
    onMouseEnter: (e: MouseEvent<HTMLElement>) => {
      const existing = child.props?.onMouseEnter;
      if (typeof existing === "function") {
        existing(e);
      }
      handleMouseEnter(e);
    },
    onMouseLeave: (e: MouseEvent<HTMLElement>) => {
      const existing = child.props?.onMouseLeave;
      if (typeof existing === "function") {
        existing(e);
      }
      handleMouseLeave();
    },
    onMouseMove: (e: MouseEvent<HTMLElement>) => {
      const existing = child.props?.onMouseMove;
      if (typeof existing === "function") {
        existing(e);
      }
      // Update tooltip position on mouse move
      handleMouseMove(e);
    },
  });

  return (
    <>
      {clonedChild}
      {isVisible && (
        <div
          className="fixed z-50 pointer-events-none"
          style={{
            left: `${position.x}px`,
            top: `${position.y}px`,
            transform: "translateX(-50%)",
          }}
          onMouseEnter={() => {
            if (timeoutRef.current) {
              clearTimeout(timeoutRef.current);
              timeoutRef.current = null;
            }
          }}
          onMouseLeave={handleMouseLeave}
        >
          <div className="bg-gray-900 dark:bg-navy-800 text-white text-xs rounded-lg shadow-xl p-3 max-w-xs border border-gray-700 dark:border-navy-600">
            {content}
          </div>
          <div className="absolute left-1/2 -translate-x-1/2 -top-1 w-0 h-0 border-l-4 border-r-4 border-b-4 border-l-transparent border-r-transparent border-b-gray-900 dark:border-b-navy-800" />
        </div>
      )}
    </>
  );
}
