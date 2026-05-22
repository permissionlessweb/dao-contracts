# DAO Calendar

On-chain event scheduling, group-based authorization, and time-triggered governance for DAOs.

---

## Overview

DAO Calendar is a proposal module that brings **time-aware governance** to DAOs. Instead of proposals being abstract votes, calendar events are first-class on-chain entities with start times, end times, managing groups, and — critically — **gauge messages that automatically execute through the DAO when events begin and end**.

Think of it as a cron scheduler crossed with a governance module: events represent things that happen over time, groups control who can manage those events, and gauges are the actions that fire when the calendar says so.

### Why a Calendar?

Most DAO governance is reactive — someone submits a proposal, people vote, it executes. But many DAO operations are **temporal**:

- Recurring reward distributions that start and stop on schedule
- Seasonal governance periods (e.g., "budget season")
- Time-boxed delegation windows
- Event-driven gauge weight adjustments
- Coordinated multi-contract state changes at known future times

The calendar module makes these patterns first-class. An event isn't just a record — it's a **trigger** that can execute DAO-level messages when its start time and end time are reached.

 