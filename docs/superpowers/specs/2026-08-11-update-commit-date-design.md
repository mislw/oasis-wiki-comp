# Update Commit Date Design

## Goal

Replace the GitHub update page's latest commit SHA with the latest commit's calendar date.

## Behavior

When the configured repository has no GitHub Release, the backend reads the default branch's latest commit timestamp from the existing GitHub commit response. It persists that ISO timestamp with the existing update status and returns it to the React window.

The update page formats that timestamp in the local browser timezone as `YYYY-MM-DD`. It renders `Latest commit: YYYY-MM-DD` when the timestamp exists, and keeps the current revision text as a fallback for incomplete or older saved settings. Release status remains version-based.

## Safety

The latest revision SHA remains the comparison key and installed revision value. The one-click update path continues to download the default branch zipball and install it to selected Agent targets without behavior changes.

## Verification

Add a Rust regression test proving that the nested GitHub commit author date is deserialized. Run the Rust test suite and the TypeScript production build.
