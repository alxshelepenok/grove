export const HIT_TOLERANCE_PX = 4;

export const screenHitRadius = (worldRadius, zoom) => worldRadius * zoom + HIT_TOLERANCE_PX;

export const pickSearchRadius = (maxWorldRadius, zoom) => maxWorldRadius + HIT_TOLERANCE_PX / zoom;
