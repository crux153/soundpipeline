#!/usr/bin/env node

const fs = require("fs");

function parseTimeToSeconds(timeStr, timeFormat = 'decimal') {
  // Check if this is samples format
  if (timeFormat.includes('Hz')) {
    const sampleRate = parseInt(timeFormat.split(' ')[0]);
    const samples = parseInt(timeStr);
    return samples / sampleRate;
  }
  
  // Parse MM:SS.sss/MM:SS.ssssss or H:MM:SS.sss/H:MM:SS.ssssss format to seconds (decimal format)
  const parts = timeStr.split(":");
  let seconds = 0;

  if (parts.length === 2) {
    // MM:SS.sss or MM:SS.ssssss format
    const [minutes, secondsAndMs] = parts;
    seconds = parseInt(minutes) * 60 + parseFloat(secondsAndMs);
  } else if (parts.length === 3) {
    // H:MM:SS.sss or H:MM:SS.ssssss format
    const [hours, minutes, secondsAndMs] = parts;
    seconds =
      parseInt(hours) * 3600 +
      parseInt(minutes) * 60 +
      parseFloat(secondsAndMs);
  }

  return seconds;
}

function secondsToTimeFormat(seconds) {
  // Convert seconds to h:mm:ss.SSSSSS format (microsecond precision)
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  const h = hours;
  const mm = minutes.toString().padStart(2, "0");
  const ss = secs.toFixed(6).padStart(9, "0"); // 6 decimal places for microseconds

  return `${h}:${mm}:${ss}`;
}

function sanitizeFilename(name) {
  // Remove only characters that are truly problematic for filenames
  return name
    .replace(/[<>:"/\\|?*]/g, "") // Remove invalid filename characters
    .replace(/\s+/g, " ") // Normalize multiple spaces to single space
    .trim(); // Remove leading/trailing spaces
}

function convertCsvToYaml(csvPath, outputPath = null) {
  try {
    // Read CSV file
    const csvData = fs.readFileSync(csvPath, "utf8");
    const lines = csvData.split("\n").filter((line) => line.trim());

    if (lines.length < 2) {
      throw new Error("CSV file must have at least a header and one data row");
    }

    // Parse header
    const header = lines[0].split("\t");
    const nameIndex = header.findIndex((col) =>
      col.toLowerCase().includes("name")
    );
    const startIndex = header.findIndex((col) =>
      col.toLowerCase().includes("start")
    );
    const durationIndex = header.findIndex((col) =>
      col.toLowerCase().includes("duration")
    );
    const timeFormatIndex = header.findIndex((col) =>
      col.toLowerCase().includes("time format")
    );

    if (nameIndex === -1 || startIndex === -1 || durationIndex === -1) {
      throw new Error("CSV must have Name, Start, and Duration columns");
    }

    // Parse data rows
    const tracks = [];

    for (let i = 1; i < lines.length; i++) {
      const cols = lines[i].split("\t");

      if (cols.length < Math.max(nameIndex, startIndex, durationIndex) + 1) {
        continue; // Skip incomplete rows
      }

      const name = cols[nameIndex].trim();
      const startTime = cols[startIndex].trim();
      const duration = cols[durationIndex].trim();
      const timeFormat = timeFormatIndex >= 0 ? cols[timeFormatIndex].trim() : 'decimal';

      if (!name || !startTime || !duration) {
        continue; // Skip empty rows
      }

      // Calculate start and end times
      const startSeconds = parseTimeToSeconds(startTime, timeFormat);
      const durationSeconds = parseTimeToSeconds(duration, timeFormat);
      const endSeconds = startSeconds + durationSeconds;

      // Generate filename from track name
      const filename = sanitizeFilename(name) + ".wav";

      tracks.push({
        file: filename,
        start: secondsToTimeFormat(startSeconds),
        end: secondsToTimeFormat(endSeconds),
      });
    }

    // Generate YAML content
    let yamlContent = "# Generated from Audition CSV markers\n";
    yamlContent += "# Split step configuration\n\n";
    yamlContent += "type: split\n";
    yamlContent += 'input: "input_audio.wav"\n';
    yamlContent += 'output_dir: "./splits"\n';
    yamlContent += "files:\n";

    tracks.forEach((track) => {
      yamlContent += `  - file: "${track.file}"\n`;
      yamlContent += `    start: "${track.start}"\n`;
      yamlContent += `    end: "${track.end}"\n`;
    });

    // Write output
    const outputFile = outputPath || csvPath.replace(/\.csv$/i, "_split.yml");
    fs.writeFileSync(outputFile, yamlContent);

    console.log(`✓ Converted ${tracks.length} tracks`);
    console.log(`✓ Output written to: ${outputFile}`);

    return outputFile;
  } catch (error) {
    console.error("Error converting CSV:", error.message);
    process.exit(1);
  }
}

// Main execution
if (require.main === module) {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    console.log("Usage: node convert-markers.js <csv-file> [output-file]");
    console.log("");
    console.log(
      "Converts Adobe Audition CSV markers to YAML split step format"
    );
    console.log("");
    console.log("Example:");
    console.log("  node convert-markers.js Markers.csv");
    console.log("  node convert-markers.js Markers.csv split_config.yml");
    process.exit(1);
  }

  const csvFile = args[0];
  const outputFile = args[1];

  if (!fs.existsSync(csvFile)) {
    console.error(`Error: File '${csvFile}' not found`);
    process.exit(1);
  }

  convertCsvToYaml(csvFile, outputFile);
}

module.exports = { convertCsvToYaml };
