export type AnalyzerBounds = { x: number; y: number; width: number; height: number };

export type AnalyzedNode = {
  id: string;
  name: string;
  category: "background" | "panel" | "button" | "text" | "artwork";
  parent_id?: string;
  bounds: AnalyzerBounds;
  extraction: {
    mode: "native" | "extract_artwork" | "reconstruct_skin" | "composite";
    target_component_id: string;
    confidence: number;
    reason: string;
    remove_content?: string[];
    transparent?: boolean;
    evaluate_nine_slice?: boolean;
  };
  z_index: number;
  node_kind: "composite" | "skin" | "artwork" | "native";
  render_mode: "bitmap" | "outline";
};

export type ImageAnalysisResult = {
  tree: {
    artifact_type: "ui_tree";
    schema_version: number;
    status: string;
    page_size: { width: number; height: number };
    nodes: AnalyzedNode[];
  };
  stats: { panels: number; buttons: number; text: number; artwork: number };
};

type PixelRegion = AnalyzerBounds & { pixels: number; fill: number; edgePixels?: number };

const MAX_ANALYSIS_SIDE = 720;

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function area(bounds: AnalyzerBounds) {
  return bounds.width * bounds.height;
}

function contains(parent: AnalyzerBounds, child: AnalyzerBounds, inset = 0) {
  return parent.x + inset <= child.x
    && parent.y + inset <= child.y
    && parent.x + parent.width - inset >= child.x + child.width
    && parent.y + parent.height - inset >= child.y + child.height;
}

function intersectionArea(left: AnalyzerBounds, right: AnalyzerBounds) {
  const width = Math.max(0, Math.min(left.x + left.width, right.x + right.width) - Math.max(left.x, right.x));
  const height = Math.max(0, Math.min(left.y + left.height, right.y + right.height) - Math.max(left.y, right.y));
  return width * height;
}

function overlapRatio(left: AnalyzerBounds, right: AnalyzerBounds) {
  return intersectionArea(left, right) / Math.max(1, Math.min(area(left), area(right)));
}

function sourceBounds(region: PixelRegion, inverseScale: number, sourceWidth: number, sourceHeight: number, padding = 0): AnalyzerBounds {
  const x = clamp(Math.floor((region.x - padding) * inverseScale), 0, sourceWidth - 1);
  const y = clamp(Math.floor((region.y - padding) * inverseScale), 0, sourceHeight - 1);
  const right = clamp(Math.ceil((region.x + region.width + padding) * inverseScale), x + 1, sourceWidth);
  const bottom = clamp(Math.ceil((region.y + region.height + padding) * inverseScale), y + 1, sourceHeight);
  return { x, y, width: right - x, height: bottom - y };
}

function smoothPixels(data: Uint8ClampedArray, width: number, height: number) {
  const result = new Uint8ClampedArray(data.length);
  const radius = 1;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;
      let count = 0;
      for (let oy = -radius; oy <= radius; oy += 1) {
        const py = y + oy;
        if (py < 0 || py >= height) continue;
        for (let ox = -radius; ox <= radius; ox += 1) {
          const px = x + ox;
          if (px < 0 || px >= width) continue;
          const offset = (py * width + px) * 4;
          r += data[offset];
          g += data[offset + 1];
          b += data[offset + 2];
          a += data[offset + 3];
          count += 1;
        }
      }
      const offset = (y * width + x) * 4;
      result[offset] = r / count;
      result[offset + 1] = g / count;
      result[offset + 2] = b / count;
      result[offset + 3] = a / count;
    }
  }
  return result;
}

function colorDistance(pixels: Uint8ClampedArray, left: number, right: number) {
  const dr = pixels[left] - pixels[right];
  const dg = pixels[left + 1] - pixels[right + 1];
  const db = pixels[left + 2] - pixels[right + 2];
  return Math.abs(dr) * 0.3 + Math.abs(dg) * 0.45 + Math.abs(db) * 0.25;
}

function findFlatRegions(pixels: Uint8ClampedArray, width: number, height: number) {
  const visited = new Uint8Array(width * height);
  const queue = new Int32Array(width * height);
  const regions: PixelRegion[] = [];
  const minimumPixels = Math.max(24, Math.floor(width * height * 0.00018));

  for (let start = 0; start < width * height; start += 1) {
    if (visited[start]) continue;
    visited[start] = 1;
    const alpha = pixels[start * 4 + 3];
    if (alpha < 20) continue;

    let head = 0;
    let tail = 0;
    queue[tail++] = start;
    let count = 0;
    let minX = width;
    let minY = height;
    let maxX = 0;
    let maxY = 0;
    let meanR = pixels[start * 4];
    let meanG = pixels[start * 4 + 1];
    let meanB = pixels[start * 4 + 2];

    while (head < tail) {
      const index = queue[head++];
      const x = index % width;
      const y = Math.floor(index / width);
      const offset = index * 4;
      count += 1;
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
      const blend = 1 / Math.min(count, 256);
      meanR += (pixels[offset] - meanR) * blend;
      meanG += (pixels[offset + 1] - meanG) * blend;
      meanB += (pixels[offset + 2] - meanB) * blend;

      const neighbors = [index - 1, index + 1, index - width, index + width];
      for (const next of neighbors) {
        if (next < 0 || next >= width * height || visited[next]) continue;
        const nx = next % width;
        if ((next === index - 1 || next === index + 1) && Math.abs(nx - x) !== 1) continue;
        const nextOffset = next * 4;
        if (pixels[nextOffset + 3] < 20) {
          visited[next] = 1;
          continue;
        }
        const meanDistance = Math.abs(pixels[nextOffset] - meanR) * 0.3
          + Math.abs(pixels[nextOffset + 1] - meanG) * 0.45
          + Math.abs(pixels[nextOffset + 2] - meanB) * 0.25;
        if (meanDistance > 23 || colorDistance(pixels, offset, nextOffset) > 30) continue;
        visited[next] = 1;
        queue[tail++] = next;
      }
    }

    if (count < minimumPixels) continue;
    const regionWidth = maxX - minX + 1;
    const regionHeight = maxY - minY + 1;
    const fill = count / (regionWidth * regionHeight);
    if (regionWidth < 5 || regionHeight < 5 || fill < 0.32) continue;
    regions.push({ x: minX, y: minY, width: regionWidth, height: regionHeight, pixels: count, fill });
  }
  return regions;
}

function edgeMask(pixels: Uint8ClampedArray, width: number, height: number) {
  const luma = new Uint8Array(width * height);
  const mask = new Uint8Array(width * height);
  for (let index = 0; index < width * height; index += 1) {
    const offset = index * 4;
    luma[index] = pixels[offset] * 0.299 + pixels[offset + 1] * 0.587 + pixels[offset + 2] * 0.114;
  }
  for (let y = 1; y < height - 1; y += 1) {
    for (let x = 1; x < width - 1; x += 1) {
      const index = y * width + x;
      const horizontal = Math.abs(luma[index + 1] - luma[index - 1]);
      const vertical = Math.abs(luma[index + width] - luma[index - width]);
      if (horizontal + vertical > 42) mask[index] = 1;
    }
  }
  return mask;
}

function dilate(source: Uint8Array, width: number, height: number, radiusX: number, radiusY: number) {
  const result = new Uint8Array(source.length);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      if (!source[y * width + x]) continue;
      for (let oy = -radiusY; oy <= radiusY; oy += 1) {
        const py = y + oy;
        if (py < 0 || py >= height) continue;
        for (let ox = -radiusX; ox <= radiusX; ox += 1) {
          const px = x + ox;
          if (px >= 0 && px < width) result[py * width + px] = 1;
        }
      }
    }
  }
  return result;
}

function connectedMaskRegions(mask: Uint8Array, edge: Uint8Array, width: number, height: number) {
  const visited = new Uint8Array(mask.length);
  const queue = new Int32Array(mask.length);
  const regions: PixelRegion[] = [];
  for (let start = 0; start < mask.length; start += 1) {
    if (!mask[start] || visited[start]) continue;
    let head = 0;
    let tail = 0;
    queue[tail++] = start;
    visited[start] = 1;
    let count = 0;
    let edgePixels = 0;
    let minX = width;
    let minY = height;
    let maxX = 0;
    let maxY = 0;
    while (head < tail) {
      const index = queue[head++];
      const x = index % width;
      const y = Math.floor(index / width);
      count += 1;
      edgePixels += edge[index];
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
      const neighbors = [index - 1, index + 1, index - width, index + width];
      for (const next of neighbors) {
        if (next < 0 || next >= mask.length || !mask[next] || visited[next]) continue;
        const nx = next % width;
        if ((next === index - 1 || next === index + 1) && Math.abs(nx - x) !== 1) continue;
        visited[next] = 1;
        queue[tail++] = next;
      }
    }
    const regionWidth = maxX - minX + 1;
    const regionHeight = maxY - minY + 1;
    if (edgePixels < 3 || regionWidth < 2 || regionHeight < 2) continue;
    regions.push({ x: minX, y: minY, width: regionWidth, height: regionHeight, pixels: count, fill: count / (regionWidth * regionHeight), edgePixels });
  }
  return regions;
}

function dedupe(regions: PixelRegion[], limit: number) {
  const selected: PixelRegion[] = [];
  for (const region of regions.sort((left, right) => area(right) - area(left))) {
    if (selected.some((item) => overlapRatio(item, region) > 0.88)) continue;
    selected.push(region);
    if (selected.length >= limit) break;
  }
  return selected;
}

function centerInside(parent: AnalyzerBounds, child: AnalyzerBounds, padding = 0) {
  const centerX = child.x + child.width / 2;
  const centerY = child.y + child.height / 2;
  return centerX >= parent.x - padding
    && centerX <= parent.x + parent.width + padding
    && centerY >= parent.y - padding
    && centerY <= parent.y + parent.height + padding;
}

function acceptsChild(parent: AnalyzedNode, child: AnalyzedNode) {
  if (parent.category === "background" || parent.id === child.id) return false;
  if (child.category === "panel" || child.category === "button") return parent.category === "panel";
  return parent.category === "panel" || parent.category === "button" || parent.category === "artwork";
}

function semanticParentPenalty(parent: AnalyzedNode, child: AnalyzedNode) {
  if (child.category === "text") {
    if (parent.category === "button") return 0;
    if (parent.category === "artwork") return 0.16;
    return 0.32;
  }
  if (child.category === "artwork") {
    if (parent.category === "button") return 0;
    if (parent.category === "artwork") return 0.12;
    return 0.24;
  }
  return 0;
}

function parentScore(parent: AnalyzedNode, child: AnalyzedNode) {
  if (!acceptsChild(parent, child) || area(parent.bounds) <= area(child.bounds) * 1.05) return Number.POSITIVE_INFINITY;
  const coverage = intersectionArea(parent.bounds, child.bounds) / Math.max(1, area(child.bounds));
  const strictlyContained = contains(parent.bounds, child.bounds, -2);
  if (!strictlyContained && (coverage < 0.72 || !centerInside(parent.bounds, child.bounds, 4))) return Number.POSITIVE_INFINITY;
  const areaRatio = area(parent.bounds) / Math.max(1, area(child.bounds));
  return Math.log(areaRatio) + semanticParentPenalty(parent, child) + (1 - coverage) * 4;
}

export function assignParents(nodes: AnalyzedNode[]) {
  const parents = nodes.filter((node) => node.category === "panel" || node.category === "button" || node.category === "artwork");
  return nodes.map((node) => {
    if (node.category === "background") return node;
    const parent = parents
      .map((candidate) => ({ candidate, score: parentScore(candidate, node) }))
      .filter((entry) => Number.isFinite(entry.score))
      .sort((left, right) => left.score - right.score)[0]?.candidate;
    return parent ? { ...node, parent_id: parent.id } : node;
  });
}

function buttonVariant(region: PixelRegion, regions: PixelRegion[]) {
  const peers = regions.filter((candidate) => {
    const widthRatio = candidate.width / region.width;
    const heightRatio = candidate.height / region.height;
    return widthRatio > 0.84 && widthRatio < 1.19 && heightRatio > 0.84 && heightRatio < 1.19;
  });
  if (peers.length < 2) return "unique";
  return `${Math.round(region.width / 8)}x${Math.round(region.height / 8)}`;
}

export async function analyzeUIImage(image: HTMLImageElement): Promise<ImageAnalysisResult> {
  const sourceWidth = image.naturalWidth || image.width;
  const sourceHeight = image.naturalHeight || image.height;
  if (!sourceWidth || !sourceHeight) throw new Error("图片尺寸无效");
  const analysisScale = Math.min(1, MAX_ANALYSIS_SIDE / Math.max(sourceWidth, sourceHeight));
  const width = Math.max(1, Math.round(sourceWidth * analysisScale));
  const height = Math.max(1, Math.round(sourceHeight * analysisScale));
  const inverseScale = 1 / analysisScale;
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) throw new Error("浏览器不支持 Canvas 图像分析");
  context.drawImage(image, 0, 0, width, height);
  const raw = context.getImageData(0, 0, width, height).data;
  const pixels = smoothPixels(raw, width, height);
  const flatRegions = findFlatRegions(pixels, width, height);
  const totalArea = width * height;

  const rectangular = flatRegions.filter((region) => region.fill >= 0.48);
  const panelRegions = dedupe(rectangular.filter((region) => {
    const ratio = area(region) / totalArea;
    return ratio >= 0.025 && ratio <= 0.82
      && region.width >= width * 0.16
      && region.height >= height * 0.09;
  }), 24);

  const buttonRegions = dedupe(rectangular.filter((region) => {
    const ratio = area(region) / totalArea;
    const aspect = region.width / region.height;
    return ratio >= 0.0007 && ratio <= 0.075
      && region.width >= 18 && region.height >= 10
      && aspect >= 0.75 && aspect <= 8
      && !panelRegions.some((panel) => overlapRatio(panel, region) > 0.92 && area(panel) < area(region) * 1.2);
  }), 56);

  const edge = edgeMask(pixels, width, height);
  const glyphRegions = connectedMaskRegions(dilate(edge, width, height, 2, 1), edge, width, height)
    .filter((region) => region.height >= 3 && region.height <= Math.max(16, height * 0.085) && region.width >= 3 && region.width <= width * 0.55);

  const lineGroups: PixelRegion[] = [];
  for (const region of glyphRegions.sort((left, right) => left.y - right.y || left.x - right.x)) {
    const centerY = region.y + region.height / 2;
    const group = lineGroups.find((candidate) => {
      const candidateCenter = candidate.y + candidate.height / 2;
      const gap = region.x > candidate.x ? region.x - (candidate.x + candidate.width) : candidate.x - (region.x + region.width);
      return Math.abs(centerY - candidateCenter) <= Math.max(region.height, candidate.height) * 0.58
        && gap <= Math.max(8, Math.max(region.height, candidate.height) * 1.35);
    });
    if (!group) {
      lineGroups.push({ ...region });
      continue;
    }
    const right = Math.max(group.x + group.width, region.x + region.width);
    const bottom = Math.max(group.y + group.height, region.y + region.height);
    group.x = Math.min(group.x, region.x);
    group.y = Math.min(group.y, region.y);
    group.width = right - group.x;
    group.height = bottom - group.y;
    group.pixels += region.pixels;
    group.edgePixels = (group.edgePixels ?? 0) + (region.edgePixels ?? 0);
    group.fill = group.pixels / area(group);
  }

  const textRegions = dedupe(lineGroups.filter((region) => {
    const aspect = region.width / region.height;
    const density = (region.edgePixels ?? 0) / Math.max(1, area(region));
    return region.width >= 7 && region.height >= 3 && aspect >= 0.45 && density >= 0.025
      && !buttonRegions.some((button) => overlapRatio(button, region) > 0.94 && area(button) < area(region) * 1.25);
  }), 120);

  const artworkRegions = dedupe(connectedMaskRegions(dilate(edge, width, height, 1, 1), edge, width, height)
    .filter((region) => {
      const ratio = area(region) / totalArea;
      const aspect = region.width / region.height;
      return ratio >= 0.00025 && ratio <= 0.045
        && region.width >= 8 && region.height >= 8
        && aspect >= 0.35 && aspect <= 2.8
        && !textRegions.some((text) => overlapRatio(text, region) > 0.68)
        && !buttonRegions.some((button) => overlapRatio(button, region) > 0.9);
    }), 80);

  const nodes: AnalyzedNode[] = [{
    id: "background.root",
    name: "背景",
    category: "background",
    bounds: { x: 0, y: 0, width: sourceWidth, height: sourceHeight },
    extraction: {
      mode: "reconstruct_skin",
      target_component_id: "background.page",
      confidence: 1,
      reason: "整张底图作为独立背景层；其上的文字、按钮和图标不会合并进该节点。",
      remove_content: ["all_controls", "all_text", "all_icons"],
      transparent: false,
    },
    z_index: 0,
    node_kind: "skin",
    render_mode: "bitmap",
  }];

  panelRegions.forEach((region, index) => {
    nodes.push({
      id: `panel.auto.${String(index + 1).padStart(2, "0")}`,
      name: `自动面板 ${index + 1}`,
      category: "panel",
      bounds: sourceBounds(region, inverseScale, sourceWidth, sourceHeight, 2),
      extraction: {
        mode: "reconstruct_skin",
        target_component_id: `panel.auto.${index + 1}`,
        confidence: clamp(0.55 + region.fill * 0.35, 0, 0.94),
        reason: "检测到大面积连续矩形区域，建立为容器，并把内部控件挂到该层级。",
        remove_content: ["text", "buttons", "icons"],
        transparent: true,
        evaluate_nine_slice: true,
      },
      z_index: 10 + index,
      node_kind: "composite",
      render_mode: "outline",
    });
  });

  buttonRegions.forEach((region, index) => {
    const variant = buttonVariant(region, buttonRegions);
    nodes.push({
      id: `button.auto.${String(index + 1).padStart(2, "0")}`,
      name: `自动按钮 ${index + 1}`,
      category: "button",
      bounds: sourceBounds(region, inverseScale, sourceWidth, sourceHeight, 2),
      extraction: {
        mode: "reconstruct_skin",
        target_component_id: `button.auto.${variant}`,
        confidence: clamp(0.54 + region.fill * 0.36, 0, 0.94),
        reason: variant === "unique" ? "检测到独立矩形交互外观。" : "检测到尺寸重复的按钮或页签，复用同一皮肤变体。",
        remove_content: ["label", "icon", "counter"],
        transparent: true,
        evaluate_nine_slice: true,
      },
      z_index: 100 + index,
      node_kind: "composite",
      render_mode: "outline",
    });
  });

  textRegions.forEach((region, index) => {
    nodes.push({
      id: `text.auto.${String(index + 1).padStart(2, "0")}`,
      name: `原生文字 ${index + 1}`,
      category: "text",
      bounds: sourceBounds(region, inverseScale, sourceWidth, sourceHeight, 1),
      extraction: {
        mode: "native",
        target_component_id: `text.auto.${index + 1}`,
        confidence: 0.72,
        reason: "检测到连续文字行或数字区域，保留为可编辑的原生 TextBlock。",
      },
      z_index: 300 + index,
      node_kind: "native",
      render_mode: "outline",
    });
  });

  artworkRegions.forEach((region, index) => {
    nodes.push({
      id: `artwork.auto.${String(index + 1).padStart(2, "0")}`,
      name: `图标或装饰 ${index + 1}`,
      category: "artwork",
      bounds: sourceBounds(region, inverseScale, sourceWidth, sourceHeight, 2),
      extraction: {
        mode: "extract_artwork",
        target_component_id: `artwork.auto.${index + 1}`,
        confidence: 0.64,
        reason: "检测到独立图标或装饰轮廓；Source Crop 仅作候选，仍需净化透明背景。",
        transparent: true,
      },
      z_index: 200 + index,
      node_kind: "artwork",
      render_mode: "bitmap",
    });
  });

  const layeredNodes = assignParents(nodes);
  return {
    tree: {
      artifact_type: "ui_tree",
      schema_version: 2,
      status: "auto_detected_candidate",
      page_size: { width: sourceWidth, height: sourceHeight },
      nodes: layeredNodes,
    },
    stats: { panels: panelRegions.length, buttons: buttonRegions.length, text: textRegions.length, artwork: artworkRegions.length },
  };
}
