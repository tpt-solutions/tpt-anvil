export function sumArray(nums: number[]): number {
  let total = 0;
  for (const n of nums) {
    total += n;
  }
  return total;
}

export function capitalize(s: string): string {
  return s.toUpperCase();
}

export function findMax(nums: number[]): number | null {
  if (nums.length === 0) return null;
  let max = nums[0];
  for (const n of nums) {
    if (n > max) max = n;
  }
  return max;
}
